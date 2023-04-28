use std::{
    mem::MaybeUninit,
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{Duration, SystemTime},
};

use bytes_kman::TBytes;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::common::{
    adress::Adress,
    packets::{
        InfoRequest, Packets, Register, RegisterResponse, Request, RequestFinal, RequestResponse,
        Search,
    },
};

use super::response::{self, NewRequestFinal, RequestStage, Response};

#[derive(Debug)]
pub enum ConnectionError {
    InvalidIp,
    HostIsNotAlive,
    InvalidInfo,
    InvalidAdress,
}

pub struct Connection {
    pub session: usize,
    pub conn: Socket,
    pub adress: SocketAddr,
    pub info: ConnectionInfo,
    pub last_packet: SystemTime,
    pub packets: Vec<Packets>,
    pub adresses: Vec<Adress>,
}

#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    pub client: String,
    pub name: String,
    pub public: Vec<u8>,
    pub other: Vec<u8>,
    pub privacy: bool,
}

impl Connection {
    pub fn new(ip: impl Into<String>, info: ConnectionInfo) -> Result<Self, ConnectionError> {
        let Ok(mut adress) = format!("{}:2120", ip.into())
            .to_socket_addrs() else {return Err(ConnectionError::InvalidIp)};
        let Some(adress) = adress.next() else{return Err(ConnectionError::InvalidIp)};
        let address_sock = SockAddr::from(adress);
        let conn = Socket::new(
            Domain::for_address(adress),
            Type::STREAM,
            Some(Protocol::TCP),
        )
        .unwrap();
        if conn.connect(&address_sock).is_err() {
            return Err(ConnectionError::HostIsNotAlive);
        }

        let local_addr = conn.local_addr().unwrap().as_socket().unwrap().ip();

        let pak = Packets::Register(Register::Client {
            client: info.client.clone(),
            public: info.public.clone(),
            name: info.name.clone(),
            other: info.other.clone(),
            privacy: info.privacy,
            private_adress: local_addr.to_string(),
        });

        let mut bytes = pak.to_bytes();
        bytes.reverse();

        let Ok(_) = conn.send(&bytes) else {
            return Err(ConnectionError::InvalidInfo);
        };

        let mut buffer = [MaybeUninit::new(0); 1024];

        let Ok(len) = conn.recv(&mut buffer)else{
            return Err(ConnectionError::InvalidInfo);
        };

        let buffer: &[u8] = unsafe { std::mem::transmute(&buffer[0..len]) };
        let mut buffer = buffer.to_owned();
        let Some(packet) = Packets::from_bytes(&mut buffer) else{return Err(ConnectionError::InvalidInfo)};
        let Packets::RegisterResponse(res) = packet else {return Err(ConnectionError::InvalidInfo)};

        let RegisterResponse::Client{ accepted, session } = res else {
            return Err(ConnectionError::InvalidAdress);
        };
        if !accepted {
            return Err(ConnectionError::InvalidAdress);
        }

        conn.set_nonblocking(true).unwrap();
        let _ = conn.set_recv_buffer_size(1024);
        let _ = conn.set_send_buffer_size(1024);

        Ok(Self {
            session,
            conn,
            info,
            last_packet: SystemTime::now(),
            packets: Vec::new(),
            adresses: Vec::new(),
            adress,
        })
    }

    pub fn step(&mut self) {
        if let Some(packet) = self.recv() {
            if let Packets::SearchResponse(pak) = &packet {
                self.adresses = pak.adresses.clone()
            };
            self.packets.push(packet)
        }

        if self.last_packet.elapsed().unwrap() < Duration::from_secs(2) {
            return;
        }

        let pak = Packets::Tick {
            session: self.session,
        };
        let mut bytes = pak.to_bytes();
        bytes.reverse();
        let _ = self.conn.send(&bytes);
        self.last_packet = SystemTime::now();
    }

    pub fn send(&mut self, packet: Packets) {
        let mut packet = packet;

        // Set session for packages
        match &mut packet {
            Packets::UnRegister(pak) => pak.session = self.session,
            Packets::Search(pak) => pak.session = self.session,
            Packets::InfoRequest(pak) => pak.session = self.session,
            Packets::Request(pak) => pak.session = self.session,
            Packets::RequestResponse(pak) => pak.session = self.session,
            Packets::RequestFinal(pak) => pak.session = self.session,
            _ => {}
        }

        let mut bytes = packet.to_bytes();
        bytes.reverse();
        let _ = self.conn.send(&bytes);
        self.last_packet = SystemTime::now()
    }

    pub fn recv(&self) -> Option<Packets> {
        let mut buffer = [MaybeUninit::new(0); 1024];
        if let Ok(len) = self.conn.recv(&mut buffer) {
            let buffer: &[u8] = unsafe { std::mem::transmute(&buffer[0..len]) };
            let mut buffer = buffer.to_owned();
            if let Some(packet) = Packets::from_bytes(&mut buffer) {
                return Some(packet);
            }
        }
        None
    }
}

pub trait TConnection {
    fn step(&self);

    fn read(&self) -> LockResult<RwLockReadGuard<Connection>>;
    fn write(&self) -> LockResult<RwLockWriteGuard<Connection>>;

    fn search(&self, search: Search) -> Response<Box<dyn TConnection>, response::SearchResponse>;
    fn info(&self, adress: &Adress) -> Response<Box<dyn TConnection>, Option<ConnectionInfo>>;

    fn request(
        &self,
        adress: &Adress,
        secret: String,
    ) -> Response<Box<dyn TConnection>, response::NewRequestResponse>;
    fn request_response(
        &self,
        adress: &Adress,
        accept: bool,
    ) -> Response<Box<dyn TConnection>, response::NewRequestFinal>;

    /// `time_offset` should be in nanosecconds
    fn request_final(
        &self,
        adress: &Adress,
        accept: bool,
        time_offset: Option<u128>,
    ) -> Response<Box<dyn TConnection>, response::ConnectOn>;
    fn add_socket(&self, socket: &Socket) -> response::RegisterResponse;

    fn adress(&self) -> Adress;

    fn has_new(&self) -> Option<RequestStage>;
    fn c(&self) -> Box<dyn TConnection + Send>;
}

impl TConnection for Arc<RwLock<Connection>> {
    fn step(&self) {
        self.write().unwrap().step();
    }

    fn read(&self) -> LockResult<RwLockReadGuard<Connection>> {
        RwLock::read(self)
    }

    fn write(&self) -> LockResult<RwLockWriteGuard<Connection>> {
        RwLock::write(self)
    }

    fn search(&self, search: Search) -> Response<Box<dyn TConnection>, response::SearchResponse> {
        let pak = Packets::Search(search);
        self.write().unwrap().send(pak.clone());

        Response {
            connection: Box::new(self.clone()),
            packets: pak,
            fn_has: search_fn_has,
            fn_get: search_fn_get,
        }
    }

    fn info(&self, adress: &Adress) -> Response<Box<dyn TConnection>, Option<ConnectionInfo>> {
        let pak = Packets::InfoRequest(InfoRequest {
            adress: adress.clone(),
            session: 0,
        });
        self.write().unwrap().send(pak.clone());

        Response {
            connection: Box::new(self.clone()),
            packets: pak,
            fn_has: info_fn_has,
            fn_get: info_fn_get,
        }
    }

    fn request(
        &self,
        adress: &Adress,
        secret: String,
    ) -> Response<Box<dyn TConnection>, response::NewRequestResponse> {
        let pak = Packets::Request(Request {
            session: 0,
            to: adress.clone(),
            secret,
        });
        self.write().unwrap().send(pak.clone());

        Response {
            connection: Box::new(self.clone()),
            packets: pak,
            fn_has: request_fn_has,
            fn_get: request_fn_get,
        }
    }

    fn request_response(
        &self,
        adress: &Adress,
        accept: bool,
    ) -> Response<Box<dyn TConnection>, NewRequestFinal> {
        let pak = Packets::RequestResponse(RequestResponse {
            session: 0,
            to: adress.clone(),
            accepted: accept,
            secret: String::new(),
        });
        self.write().unwrap().send(pak.clone());

        Response {
            connection: Box::new(self.clone()),
            packets: pak,
            fn_has: request_response_fn_has,
            fn_get: request_response_fn_get,
        }
    }

    fn request_final(
        &self,
        adress: &Adress,
        accept: bool,
        time_offset: Option<u128>,
    ) -> Response<Box<dyn TConnection>, response::ConnectOn> {
        let time_offset = match time_offset {
            Some(s) => s,
            None => Duration::from_secs(1).as_nanos(),
        };

        let pak = Packets::RequestFinal(RequestFinal {
            session: 0,
            to: adress.clone(),
            accepted: accept,
            time_offset,
        });
        self.write().unwrap().send(pak.clone());
        Response {
            connection: self.c(),
            packets: pak,
            fn_has: request_final_fn_has,
            fn_get: request_final_fn_get,
        }
    }

    fn add_socket(&self, socket: &Socket) -> response::RegisterResponse {
        let session = self.read().unwrap().session;
        let pak = Packets::Register(Register::Port { session });
        let mut bytes = pak.to_bytes();
        bytes.reverse();
        let addr = self.read().unwrap().adress.clone();
        println!("Adress: {}", addr);
        socket.connect(&addr.into());
        socket.send_to(&bytes, &addr.into());

        socket.set_nonblocking(false);
        let mut buffer = [MaybeUninit::uninit(); 4096];
        if let Ok(len) = socket.recv(&mut buffer) {
            socket.set_nonblocking(true);
            let mut buffer = buffer[0..len].to_vec();
            let mut buffer = unsafe { std::mem::transmute(buffer) };
            let Some(packet) = Packets::from_bytes(&mut buffer)else{return response::RegisterResponse::Error};
            if let Packets::RegisterResponse(res) = packet {
                match res {
                    RegisterResponse::Client { accepted, session } => {
                        return response::RegisterResponse::Error
                    }
                    RegisterResponse::Port { port } => {
                        return response::RegisterResponse::Success { port }
                    }
                }
            }
        }
        response::RegisterResponse::Error
    }

    fn adress(&self) -> Adress {
        self.read().unwrap().info.public.clone()
    }

    fn has_new(&self) -> Option<RequestStage> {
        let mut res = None;

        self.write().unwrap().packets.retain(|pak| {
            if res.is_none() {
                match pak {
                    Packets::NewRequest(pak) => {
                        res = Some(RequestStage::NewRequest(response::NewRequest {
                            connection: Box::new(self.clone()),
                            from: pak.from.clone(),
                            secret: pak.secret.clone(),
                        }));
                        false
                    }
                    Packets::NewRequestResponse(pak) => {
                        res = Some(RequestStage::NewRequestResponse(
                            response::NewRequestResponse {
                                connection: Box::new(self.clone()),
                                from: pak.from.clone(),
                                accept: pak.accepted,
                                secret: pak.secret.clone(),
                            },
                        ));
                        false
                    }
                    Packets::NewRequestFinal(pak) => {
                        res = Some(RequestStage::NewRequestFinal(response::NewRequestFinal {
                            connection: Box::new(self.clone()),
                            from: pak.from.clone(),
                            accept: pak.accepted,
                        }));
                        false
                    }
                    Packets::ConnectOn(pak) => {
                        res = Some(RequestStage::ConnectOn(response::ConnectOn {
                            connection: self.c(),
                            adress: pak.adress.clone(),
                            to: pak.to.clone(),
                            port: pak.port,
                            time: pak.time,
                        }));
                        false
                    }
                    _ => true,
                }
            } else {
                true
            }
        });

        res
    }

    fn c(&self) -> Box<dyn TConnection + Send> {
        Box::new(self.clone())
    }
}

unsafe impl Send for Connection {}

// Search

fn search_fn_has(conn: &Box<dyn TConnection>, _: &Packets) -> bool {
    conn.step();
    for pak in conn.read().unwrap().packets.iter() {
        if let Packets::SearchResponse(_) = pak {
            return true;
        }
    }
    false
}

fn search_fn_get(conn: Box<dyn TConnection>, _: Packets) -> response::SearchResponse {
    let mut res = None;

    conn.write().unwrap().packets.retain(|pak| {
        if let Packets::SearchResponse(pak) = pak {
            if res.is_none() {
                res = Some(response::SearchResponse {
                    adresses: pak.adresses.clone(),
                });
                return false;
            }
        }
        true
    });

    if let Some(res) = res {
        res
    } else {
        panic!()
    }
}

// End Search
//
// Info

fn info_fn_has(conn: &Box<dyn TConnection>, packet: &Packets) -> bool {
    conn.step();
    if let Packets::InfoRequest(packet) = packet {
        for pak in conn.read().unwrap().packets.iter() {
            if let Packets::Info(pak) = pak {
                if pak.adress == packet.adress {
                    return true;
                }
            }
        }
    }
    false
}

fn info_fn_get(conn: Box<dyn TConnection>, packet: Packets) -> Option<ConnectionInfo> {
    let mut res = None;
    if let Packets::InfoRequest(packet) = packet {
        conn.write().unwrap().packets.retain(|pak| {
            if let Packets::Info(pak) = pak {
                if pak.adress == packet.adress && res.is_none() {
                    if pak.has {
                        res = Some(Some(ConnectionInfo {
                            client: pak.client.clone(),
                            name: pak.client.clone(),
                            public: packet.adress.clone(),
                            other: pak.other.clone(),
                            privacy: false,
                        }));
                    } else {
                        res = Some(None)
                    }
                    return false;
                }
            }
            true
        })
    }

    if let Some(res) = res {
        res
    } else {
        panic!()
    }
}

// End Info
//
// Request

fn request_fn_has(conn: &Box<dyn TConnection>, packet: &Packets) -> bool {
    conn.step();
    if let Packets::Request(packet) = packet {
        for pak in conn.read().unwrap().packets.iter() {
            if let Packets::NewRequestResponse(pak) = pak {
                if pak.from == packet.to {
                    return true;
                }
            }
        }
    }
    false
}

fn request_fn_get(conn: Box<dyn TConnection>, packet: Packets) -> response::NewRequestResponse {
    let mut res = None;
    if let Packets::Request(packet) = packet {
        conn.write().unwrap().packets.retain(|pak| {
            if let Packets::NewRequestResponse(pak) = pak {
                if pak.from == packet.to && res.is_none() {
                    res = Some(response::NewRequestResponse {
                        connection: conn.c(),
                        from: pak.from.clone(),
                        accept: pak.accepted,
                        secret: pak.secret.clone(),
                    });
                    return false;
                }
            }
            true
        })
    }

    if let Some(res) = res {
        res
    } else {
        panic!()
    }
}

// End Request
//
// RequestResponse

fn request_response_fn_has(conn: &Box<dyn TConnection>, packet: &Packets) -> bool {
    conn.step();
    if let Packets::Request(packet) = packet {
        for pak in conn.read().unwrap().packets.iter() {
            if let Packets::NewRequestFinal(pak) = pak {
                if pak.from == packet.to {
                    return true;
                }
            }
        }
    }
    false
}

fn request_response_fn_get(
    conn: Box<dyn TConnection>,
    packet: Packets,
) -> response::NewRequestFinal {
    let mut res = None;
    if let Packets::Request(packet) = packet {
        conn.write().unwrap().packets.retain(|pak| {
            if let Packets::NewRequestFinal(pak) = pak {
                if pak.from == packet.to && res.is_none() {
                    res = Some(response::NewRequestFinal {
                        connection: conn.c(),
                        from: pak.from.clone(),
                        accept: pak.accepted,
                    });
                    return false;
                }
            }
            true
        })
    }

    if let Some(res) = res {
        res
    } else {
        panic!()
    }
}

// End RequestResponse
//
// RequestFinal

fn request_final_fn_has(conn: &Box<dyn TConnection>, packet: &Packets) -> bool {
    conn.step();
    if let Packets::RequestFinal(packet) = packet {
        for pak in conn.read().unwrap().packets.iter() {
            if let Packets::ConnectOn(pak) = pak {
                if pak.adress == packet.to {
                    return true;
                }
            }
        }
    }
    false
}

fn request_final_fn_get(conn: Box<dyn TConnection>, packet: Packets) -> response::ConnectOn {
    let mut res = None;
    if let Packets::RequestFinal(packet) = packet {
        conn.write().unwrap().packets.retain(|pak| {
            if let Packets::ConnectOn(pak) = pak {
                if pak.adress == packet.to && res.is_none() {
                    res = Some(response::ConnectOn {
                        connection: conn.c(),
                        adress: pak.adress.clone(),
                        to: pak.to.clone(),
                        port: pak.port,
                        time: pak.time,
                    });
                    return false;
                }
            }
            true
        })
    }

    if let Some(res) = res {
        res
    } else {
        panic!()
    }
}
