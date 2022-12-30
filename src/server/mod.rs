mod connect;
mod on_info;
mod on_request;
mod on_request_final;
mod on_request_response;
mod on_search;

use bytes_kman::TBytes;
use polling::{Event, Poller};
use rand::random;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

// RelayServer allways should be on this port
pub const PORT: u16 = 2120;

use crate::common::{adress::Adress, packets::*};
use std::{
    mem::MaybeUninit,
    net::ToSocketAddrs,
    os::fd::{self, FromRawFd, IntoRawFd, RawFd},
    time::{Duration, SystemTime},
};

#[derive(PartialEq, Clone, Debug)]
pub enum Connecting {
    Start(usize),
    Finishing(usize),
}
impl Connecting {
    pub fn session(&self) -> usize {
        match self {
            Connecting::Start(s) => *s,
            Connecting::Finishing(s) => *s,
        }
    }
}

#[derive(Debug)]
pub enum ClientStage {
    NotRegistered,
    Registered(RegisteredClient),
}

#[derive(Debug)]
pub struct RegisteredClient {
    pub name: String,
    pub client: String,
    pub other: Vec<u8>,
    pub adress: Adress,
    pub ports: Vec<u16>,
    pub to_connect: Vec<Connecting>,
    pub privacy: bool,
}

#[derive(Debug)]
pub struct Client {
    pub session: usize,
    pub conn: Socket,
    pub fd: RawFd,
    pub from: SockAddr,
    pub stage: ClientStage,
    pub last_message: SystemTime,
    pub buffer: Vec<MaybeUninit<u8>>,
}

#[derive(Debug)]
pub struct RelayServer {
    pub clients: Vec<Client>,
    pub poller: Poller,
    pub conn: Socket,
    pub fd: RawFd,
    pub buffer: Vec<MaybeUninit<u8>>,
    pub client_timeout: Duration,
    pub connect_warmup: Duration,
}

impl RelayServer {
    pub fn new(client_timeout: Duration, connect_warmup: Duration) -> Result<Self, ()> {
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

        let Ok(poller) = Poller::new() else{
            println!("Cannot create poller!");
            return Err(())
        };

        let fd = conn.into_raw_fd();
        poller.add(fd, Event::readable(0)).unwrap();
        let conn = unsafe { Socket::from_raw_fd(fd) };

        let mut buffer = Vec::new();
        buffer.resize(1024, MaybeUninit::new(0));

        Ok(Self {
            clients: Vec::new(),
            poller,
            buffer,
            fd,
            client_timeout,
            connect_warmup,
            conn,
        })
    }

    pub fn avalibile_adress(&self, adress: &Adress) -> bool {
        for client in self.clients.iter() {
            if let ClientStage::Registered(client) = &client.stage {
                if client.adress == *adress {
                    return false;
                }
            }
        }
        true
    }

    pub fn create_session(&self) -> usize {
        let mut session = random();

        'l: loop {
            if session == 0 {
                session = random();
                continue 'l;
            }

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

    pub fn listen(&mut self) {
        let mut events = Vec::new();
        let Ok(_) = self.poller.wait(&mut events, None) else {return};

        println!("Events: {:?}", events);

        for event in events {
            if event.key == 0 {
                self.accept_new();
                self.poller.modify(self.fd, Event::readable(0)).unwrap();
            } else {
                if let Some(fd) = self.process_client(event.key) {
                    self.poller.modify(fd, Event::readable(event.key)).unwrap();
                }
            }
        }
    }

    pub fn accept_new(&mut self) {
        if let Ok((conn, from)) = self.conn.accept() {
            let _ = conn.set_nonblocking(true);
            let fd = conn.into_raw_fd();
            let session = self.create_session();
            self.poller.add(fd, Event::readable(session)).unwrap();
            let conn = unsafe { Socket::from_raw_fd(fd) };

            let client = Client {
                session,
                fd,
                conn,
                from,
                stage: ClientStage::NotRegistered,
                last_message: SystemTime::now(),
                buffer: vec![MaybeUninit::new(0); 1024],
            };

            self.clients.push(client);
        }
    }

    pub fn process_client(&mut self, session: usize) -> Option<RawFd> {
        let mut to_search = None;
        let mut to_info = None;
        let mut to_request = None;
        let mut to_request_response = None;
        let mut to_request_final = None;

        let mut to_remove = false;

        let mut used_adresses = Vec::new();
        let mut index = None;
        let mut fd = None;
        for (i, client) in self.clients.iter().enumerate() {
            if let ClientStage::Registered(rclient) = &client.stage {
                used_adresses.push(rclient.adress.clone())
            }

            if client.session == session {
                index = Some(i);
                fd = Some(client.fd)
            }
        }

        let Some(index) = index else{return fd};

        if let Some(client) = self.clients.get_mut(index) {
            let Ok(len) = client.conn.recv(&mut client.buffer)else {
                client.last_message = SystemTime::UNIX_EPOCH;
                return None};
            if len == 0 {
                client.last_message = SystemTime::UNIX_EPOCH;
                return None;
            }

            println!("Len: {}", len);

            let buffer: &[u8] = unsafe { std::mem::transmute(&client.buffer[0..len]) };
            let mut buffer = buffer.to_owned();
            let Some(packet) = Packets::from_bytes(&mut buffer)else{return fd};
            match packet {
                Packets::Register(register) => {
                    if used_adresses.contains(&register.public) {
                        let pak = Packets::RegisterResponse(RegisterResponse {
                            accepted: false,
                            session: 0,
                        });
                        let mut bytes = pak.to_bytes();
                        bytes.reverse();

                        let _ = client.conn.send(&bytes);
                        return fd;
                    }

                    // Adress is valid

                    client.stage = ClientStage::Registered(RegisteredClient {
                        name: register.name,
                        client: register.client,
                        other: register.other,
                        adress: register.public,
                        ports: vec![],
                        to_connect: vec![],
                        privacy: register.privacy,
                    });

                    let pak = Packets::RegisterResponse(RegisterResponse {
                        accepted: true,
                        session: client.session,
                    });

                    let mut bytes = pak.to_bytes();
                    bytes.reverse();

                    let _ = client.conn.send(&bytes);
                }
                Packets::UnRegister(session) => {
                    if client.session == session.session {
                        to_remove = true
                    }
                }
                Packets::Search(search) => {
                    if search.session == client.session {
                        to_search = Some(search);
                        client.last_message = SystemTime::now();
                    }
                }
                Packets::InfoRequest(info) => {
                    if info.session == client.session {
                        to_info = Some(info);
                        client.last_message = SystemTime::now();
                    }
                }
                Packets::Request(request) => {
                    if request.session == client.session {
                        to_request = Some(request);
                        client.last_message = SystemTime::now();
                    }
                }
                Packets::RequestResponse(request_response) => {
                    if request_response.session == client.session {
                        to_request_response = Some(request_response);
                        client.last_message = SystemTime::now();
                    }
                }
                Packets::Avalibile(avalibile) => {
                    if avalibile.session == client.session {
                        if let ClientStage::Registered(client) = &mut client.stage {
                            client.ports.push(avalibile.port);
                        }
                        client.last_message = SystemTime::now();
                    }
                }
                Packets::RequestFinal(request_final) => {
                    if request_final.session == client.session {
                        to_request_final = Some(request_final);
                        client.last_message = SystemTime::now();
                    }
                }
                Packets::Tick { session } => {
                    if client.session == session {
                        client.last_message = SystemTime::now();
                    }
                }

                _ => {}
            }
        }

        if let Some(search) = to_search {
            self.on_search(index, search)
        }

        if let Some(info) = to_info {
            self.on_info(index, info)
        }

        if let Some(request) = to_request {
            self.on_request(index, request)
        }

        if let Some(request_response) = to_request_response {
            self.on_request_response(index, request_response)
        }

        if let Some(request_final) = to_request_final {
            self.on_request_final(index, request_final)
        }

        fd
    }

    pub fn step(&mut self) {
        self.listen();
        // self.accept_new();
        // self.process_messages();
        self.clients.retain(|client| {
            if client.last_message.elapsed().unwrap() < self.client_timeout {
                true
            } else {
                let _ = self.poller.delete(client.fd);
                false
            }
        });

        self.connect();
    }
}
