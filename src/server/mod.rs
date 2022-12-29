mod connect;
mod on_info;
mod on_request;
mod on_request_final;
mod on_request_response;
mod on_search;

use bytes_kman::TBytes;
use rand::random;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

// RelayServer allways should be on this port
pub const PORT: u16 = 2120;

use crate::common::{adress::Adress, packets::*};
use std::{mem::MaybeUninit, net::ToSocketAddrs, time::SystemTime};

#[derive(PartialEq, Clone, Debug)]
pub enum Connecting {
    Start(u128),
    Finishing(u128),
}
impl Connecting {
    pub fn session(&self) -> u128 {
        match self {
            Connecting::Start(s) => *s,
            Connecting::Finishing(s) => *s,
        }
    }
}

#[derive(Debug)]
pub struct Client {
    pub session: u128,
    pub name: String,
    pub client: String,
    pub other: Vec<u8>,
    pub adress: Adress,
    pub conn: Socket,
    pub ports: Vec<u16>,
    pub to_connect: Vec<Connecting>,
    pub last_message: SystemTime,
    pub privacy: bool,
    pub buffer: Vec<MaybeUninit<u8>>,
    pub from: SockAddr,
}

#[derive(Debug)]
pub struct RelayServer {
    pub clients: Vec<Client>,
    pub conn: Socket,
    pub buffer: Vec<MaybeUninit<u8>>,
}

impl RelayServer {
    pub fn new() -> Result<Self, ()> {
        let adress = format!("localhost:{}", PORT);
        let adress = adress.to_socket_addrs().unwrap().next().unwrap();
        let adress_sock = SockAddr::from(adress);
        let conn = Socket::new(
            Domain::for_address(adress),
            Type::STREAM,
            Some(Protocol::TCP),
        )
        .unwrap();
        let _ = conn.set_nonblocking(true).unwrap();
        let _ = conn.bind(&adress_sock).unwrap();
        let _ = conn.listen(128).unwrap();

        let mut buffer = Vec::new();
        buffer.resize(1024, MaybeUninit::new(0));

        Ok(Self {
            clients: Vec::new(),
            conn,
            buffer,
        })
    }

    pub fn avalibile_adress(&self, adress: &Adress) -> bool {
        for client in self.clients.iter() {
            if client.adress == *adress {
                return false;
            }
        }
        true
    }

    pub fn create_session(&self) -> u128 {
        let mut session = random();

        'l: loop {
            for client in self.clients.iter() {
                if client.session == session {
                    session = random();
                    continue 'l;
                }
            }
            break;
        }

        session
    }

    pub fn accept_new(&mut self) {
        if let Ok((conn, from)) = self.conn.accept() {
            let _ = conn.set_nonblocking(false).unwrap();
            if let Ok(len) = conn.recv(&mut self.buffer) {
                let buffer: &[u8] = unsafe { std::mem::transmute(&self.buffer[0..len]) };
                let mut buffer = buffer.to_owned();

                if let Some(packet) = Packets::from_bytes(&mut buffer) {
                    if let Packets::Register(register) = packet {
                        // Validate Adress
                        if !self.avalibile_adress(&register.public) {
                            let pak = Packets::RegisterResponse(RegisterResponse {
                                accepted: false,
                                session: 0,
                            });
                            let mut bytes = pak.to_bytes();
                            bytes.reverse();

                            let _ = conn.send(&bytes);
                            return;
                        }

                        // Adress is valid

                        let mut client = Client {
                            session: self.create_session(),
                            adress: register.public,
                            conn,
                            ports: Vec::new(),
                            to_connect: Vec::new(),
                            last_message: SystemTime::now(),
                            privacy: register.privacy,
                            name: register.name,
                            other: register.other,
                            buffer: Vec::new(),
                            client: register.client,
                            from,
                        };

                        client.buffer.resize(1024, MaybeUninit::new(0));

                        client
                            .conn
                            .set_nonblocking(true)
                            .expect("Connection cannot be non blocking!");

                        let pak = Packets::RegisterResponse(RegisterResponse {
                            accepted: true,
                            session: client.session,
                        });

                        let mut bytes = pak.to_bytes();
                        bytes.reverse();

                        let _ = client.conn.send(&bytes);
                        self.clients.push(client);
                    }
                }
            }
        }
    }

    pub fn process_messages(&mut self) {
        let mut to_search = Vec::new();
        let mut to_info = Vec::new();
        let mut to_request = Vec::new();
        let mut to_request_response = Vec::new();
        let mut to_request_final = Vec::new();

        let mut index = 0usize;
        self.clients.retain_mut(|client| {
            if let Ok(len) = client.conn.recv(&mut client.buffer) {
                let buffer: &[u8] = unsafe { std::mem::transmute(&client.buffer[0..len]) };
                let mut buffer = buffer.to_owned();
                if let Some(packet) = Packets::from_bytes(&mut buffer) {
                    match packet {
                        Packets::UnRegister(_) => {
                            return false;
                        }
                        Packets::Search(search) => {
                            if search.session == client.session {
                                to_search.push((index, search));
                                client.last_message = SystemTime::now();
                            }
                        }
                        Packets::InfoRequest(info) => {
                            if info.session == client.session {
                                to_info.push((index, info));
                                client.last_message = SystemTime::now();
                            }
                        }
                        Packets::Request(request) => {
                            if request.session == client.session {
                                to_request.push((index, request));
                                client.last_message = SystemTime::now();
                            }
                        }
                        Packets::RequestResponse(request_response) => {
                            if request_response.session == client.session {
                                to_request_response.push((index, request_response));
                                client.last_message = SystemTime::now();
                            }
                        }
                        Packets::Avalibile(avalibile) => {
                            if avalibile.session == client.session {
                                client.ports.push(avalibile.port);
                                client.last_message = SystemTime::now();
                            }
                        }
                        Packets::RequestFinal(request_final) => {
                            if request_final.session == client.session {
                                to_request_final.push((index, request_final));
                                client.last_message = SystemTime::now();
                            }
                        }
                        Packets::Tick { session } => {
                            if client.session == session {
                                client.last_message = SystemTime::now();
                            }
                        }
                        _ => {
                            return false;
                        }
                    }
                }
            }
            index += 1;
            true
        });

        for (index, search) in to_search {
            self.on_search(index, search)
        }

        for (index, info) in to_info {
            self.on_info(index, info)
        }

        for (index, request) in to_request {
            self.on_request(index, request)
        }

        for (index, request_response) in to_request_response {
            self.on_request_response(index, request_response)
        }

        for (index, request_final) in to_request_final {
            self.on_request_final(index, request_final)
        }
    }

    pub fn step(&mut self) {
        self.accept_new();
        self.process_messages();
        self.connect();

        self.clients
            .retain(|client| client.last_message.elapsed().unwrap().as_secs() < 5);
    }
}
