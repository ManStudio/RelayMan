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

use crate::common::{adress::Adress, packets::*, FromRawSock, IntoRawSock, RawSock};
use std::{
    mem::MaybeUninit,
    net::ToSocketAddrs,
    time::{Duration, SystemTime},
};

#[derive(PartialEq, Clone, Debug)]
pub enum Connecting {
    Start(usize),
    Finishing(usize, u128),
}
impl Connecting {
    pub fn session(&self) -> usize {
        match self {
            Connecting::Start(s) => *s,
            Connecting::Finishing(s, _) => *s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientStage {
    NotRegistered,
    Registered(RegisteredClient),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredClient {
    pub name: String,
    pub client: String,
    pub other: Vec<u8>,
    pub adress: Adress,
    pub ports: Vec<u16>,
    pub to_connect: Vec<Connecting>,
    pub privacy: bool,
    pub private_adress: String,
}

#[derive(Debug)]
pub struct Client {
    pub session: usize,
    pub conn: Socket,
    pub fd: RawSock,
    pub from: SockAddr,
    pub stage: ClientStage,
    pub last_message: SystemTime,
    pub buffer: Vec<MaybeUninit<u8>>,
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.session == other.session && self.stage == other.stage
    }
}

#[derive(Debug)]
pub struct RelayServer {
    pub clients: Vec<Client>,
    pub poller: Poller,
    pub conn: Socket,
    pub fd: RawSock,
    pub buffer: Vec<MaybeUninit<u8>>,
    pub client_timeout: Duration,
}

#[derive(Debug)]
pub enum RelayServerError {
    CannotCreatePoller,
}

impl RelayServer {
    pub fn new(ip: impl Into<String>, client_timeout: Duration) -> Result<Self, RelayServerError> {
        let adress = format!("{}:{}", ip.into(), PORT);
        let adress = adress.to_socket_addrs().unwrap().next().unwrap();
        let adress_sock = SockAddr::from(adress);
        let conn = Socket::new(
            Domain::for_address(adress),
            Type::STREAM,
            Some(Protocol::TCP),
        )
        .unwrap();

        conn.set_nonblocking(true).unwrap();
        conn.bind(&adress_sock).unwrap();
        conn.listen(128).unwrap();

        let Ok(poller) = Poller::new() else{
            println!("Cannot create poller!");
            return Err(RelayServerError::CannotCreatePoller)
        };

        let fd = conn.into_raw();
        poller.add(fd, Event::readable(0)).unwrap();
        let conn = Socket::from_raw(fd);

        let mut buffer = Vec::new();
        buffer.resize(1024, MaybeUninit::new(0));

        Ok(Self {
            clients: Vec::new(),
            poller,
            buffer,
            fd,
            client_timeout,
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

        for event in events {
            if event.key == 0 {
                self.accept_new();
                self.poller.modify(self.fd, Event::readable(0)).unwrap();
            } else if let Some(fd) = self.process_client(event.key) {
                self.poller.modify(fd, Event::readable(event.key)).unwrap();
            }
        }
    }

    pub fn accept_new(&mut self) {
        if let Ok((conn, from)) = self.conn.accept() {
            let _ = conn.set_nonblocking(true);
            let fd = conn.into_raw();
            let session = self.create_session();
            self.poller.add(fd, Event::readable(session)).unwrap();
            let conn = Socket::from_raw(fd);
            let _ = conn.set_recv_buffer_size(1024);
            let _ = conn.set_send_buffer_size(1024);

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

    pub fn process_client(&mut self, session: usize) -> Option<RawSock> {
        let mut to_search = Vec::new();
        let mut to_info = Vec::new();
        let mut to_request = Vec::new();
        let mut to_request_response = Vec::new();
        let mut to_request_final = Vec::new();

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

        loop {
            if let Some(client) = self.clients.get_mut(index) {
                let Ok(len) = client.conn.recv(&mut client.buffer)else {
                client.last_message = SystemTime::UNIX_EPOCH;
                return None};
                if len == 0 {
                    client.last_message = SystemTime::UNIX_EPOCH;
                    return None;
                }

                let buffer: &[u8] = unsafe { std::mem::transmute(&client.buffer[0..len]) };
                let mut buffer = buffer.to_owned();
                if buffer.is_empty() {
                    return fd;
                };
                let Some(packet) = Packets::from_bytes(&mut buffer)else{return fd};
                match packet {
                    Packets::Register(register) => match register {
                        Register::Client {
                            client: client_name,
                            public,
                            name,
                            other,
                            privacy,
                            private_adress,
                        } => {
                            if used_adresses.contains(&public) {
                                let pak = Packets::RegisterResponse(RegisterResponse::Client {
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
                                name,
                                client: client_name,
                                other,
                                adress: public,
                                ports: vec![],
                                to_connect: vec![],
                                privacy,
                                private_adress,
                            });

                            let pak = Packets::RegisterResponse(RegisterResponse::Client {
                                accepted: true,
                                session: client.session,
                            });

                            let mut bytes = pak.to_bytes();
                            bytes.reverse();

                            let _ = client.conn.send(&bytes);
                        }
                        Register::Port { session } => {
                            let mut pak = Packets::RegisterResponse(RegisterResponse::Client {
                                accepted: false,
                                session,
                            });
                            let Ok(conn) = client.conn.try_clone() else {break};
                            let from = client.from.clone();
                            for parent in self.clients.iter_mut() {
                                if parent.session == session {
                                    if let Some(ipv4) = from.as_socket_ipv4() {
                                        let Some(from_ipv4) = parent.from.as_socket_ipv4() else{break};
                                        if ipv4.ip() != from_ipv4.ip() {
                                            break;
                                        };
                                        if let ClientStage::Registered(registered) =
                                            &mut parent.stage
                                        {
                                            registered.ports.push(ipv4.port());
                                            pak =
                                                Packets::RegisterResponse(RegisterResponse::Port {
                                                    port: ipv4.port(),
                                                });
                                        }
                                    }
                                    break;
                                }
                            }

                            let mut bytes = pak.to_bytes();
                            bytes.reverse();

                            let _ = conn.send(&bytes);
                        }
                    },
                    Packets::UnRegister(session) => {
                        if client.session == session.session {
                            client.last_message = std::time::UNIX_EPOCH;
                        }
                    }
                    Packets::Search(search) => {
                        if search.session == client.session {
                            to_search.push(search);
                            client.last_message = SystemTime::now();
                        }
                    }
                    Packets::InfoRequest(info) => {
                        if info.session == client.session {
                            to_info.push(info);
                            client.last_message = SystemTime::now();
                        }
                    }
                    Packets::Request(request) => {
                        if request.session == client.session {
                            to_request.push(request);
                            client.last_message = SystemTime::now();
                        }
                    }
                    Packets::RequestResponse(request_response) => {
                        if request_response.session == client.session {
                            to_request_response.push(request_response);
                            client.last_message = SystemTime::now();
                        }
                    }
                    Packets::RequestFinal(request_final) => {
                        if request_final.session == client.session {
                            to_request_final.push(request_final);
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
        }

        for search in to_search {
            self.on_search(index, search)
        }

        for info in to_info {
            self.on_info(index, info)
        }

        for request in to_request {
            self.on_request(index, request)
        }

        for request_response in to_request_response {
            self.on_request_response(index, request_response)
        }

        for request_final in to_request_final {
            self.on_request_final(index, request_final)
        }

        fd
    }

    pub fn step(&mut self) {
        self.listen();
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
