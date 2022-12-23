use std::{
    mem::MaybeUninit,
    net::ToSocketAddrs,
    time::{Duration, SystemTime},
};

use bytes_kman::TBytes;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::common::{
    adress::Adress,
    packets::{InfoRequest, NewRequest, Packets, Register, Request, RequestFinal, RequestResponse},
};

#[derive(Debug)]
pub enum ConnectionError {
    InvalidIp,
    HostIsNotAlive,
    InvalidInfo,
    InvalidAdress,
}

pub struct Connection {
    pub session: u128,
    pub conn: Socket,

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

        let pak = Packets::Register(Register {
            client: info.client.clone(),
            public: info.public.clone(),
            name: info.name.clone(),
            other: info.other.clone(),
            privacy: info.privacy.clone(),
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

        if !res.accepted {
            return Err(ConnectionError::InvalidAdress);
        }

        conn.set_nonblocking(true).unwrap();

        Ok(Self {
            session: res.session,
            conn,
            info,
            last_packet: SystemTime::now(),
            packets: Vec::new(),
            adresses: Vec::new(),
        })
    }

    pub fn step(&mut self) {
        if let Some(packet) = self.recv() {
            match packet {
                Packets::SearchResponse(pak) => self.adresses = pak.adresses,
                _ => self.packets.push(packet),
            }
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
            Packets::Avalibile(pak) => pak.session = self.session,
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

    pub fn get_info_request(&mut self, adress: &Adress) {
        let pak = Packets::InfoRequest(InfoRequest {
            adress: adress.clone(),
            session: 0,
        });

        self.send(pak);
    }

    pub fn get_info(&mut self, adress: &Adress) -> Option<ConnectionInfo> {
        let mut res = None;
        self.packets.retain(|packet| match packet {
            Packets::Info(pak) => {
                println!("Pack: {:?}", pak);
                if pak.adress == *adress {
                    if pak.has {
                        res = Some(ConnectionInfo {
                            client: pak.client.clone(),
                            name: pak.name.clone(),
                            public: pak.adress.clone(),
                            other: pak.other.clone(),
                            privacy: false,
                        });
                    }
                    false
                } else {
                    true
                }
            }
            _ => true,
        });

        None
    }

    pub fn request(&mut self, adress: &Adress, secret: impl Into<String>) {
        let pak = Packets::Request(Request {
            session: 0,
            to: adress.clone(),
            secret: secret.into(),
        });

        self.send(pak);
        self.step();
    }

    pub fn request_response(&mut self, adress: &Adress, secret: Option<impl Into<String>>) {
        let accepted = secret.is_some();
        let secret = if let Some(secret) = secret {
            secret.into()
        } else {
            String::new()
        };

        let pak = Packets::RequestResponse(RequestResponse {
            session: 0,
            to: adress.clone(),
            accepted,
            secret,
        });

        self.send(pak);
        self.step();
    }

    pub fn request_final(&mut self, adress: &Adress, accepted: bool) {
        let pak = Packets::RequestFinal(RequestFinal {
            session: 0,
            adress: adress.clone(),
            accepted,
        });
        self.send(pak);
        self.step();
    }

    pub fn has_new_request(&mut self) -> Option<NewRequest> {
        let mut has = None;

        self.step();
        self.packets.retain(|packet| match packet {
            Packets::NewRequest(pak) => {
                if has.is_none() {
                    has = Some(pak.clone());
                    false
                } else {
                    true
                }
            }
            _ => true,
        });

        has
    }

    pub fn has_request_response(&mut self) -> Option<RequestResponse> {
        let mut has = None;

        self.step();

        self.packets.retain(|packet| match packet {
            Packets::RequestResponse(pak) => {
                if has.is_none() {
                    has = Some(pak.clone());
                    false
                } else {
                    true
                }
            }
            _ => true,
        });

        has
    }

    pub fn has_request_final(&mut self) -> Option<RequestFinal> {
        let mut has = None;

        self.step();

        self.packets.retain(|packet| match packet {
            Packets::RequestFinal(pak) => {
                if has.is_none() {
                    has = Some(pak.clone());
                    false
                } else {
                    true
                }
            }
            _ => true,
        });

        has
    }
}
