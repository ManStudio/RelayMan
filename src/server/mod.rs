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
            let session;
            if let Some(client) = self.clients.get(index) {
                session = client.session;
            } else {
                continue;
            }

            let mut adresses = Vec::new();

            for client in self.clients.iter() {
                let mut valid = true;

                match &search.name {
                    SearchType::Fuzzy(name) => {
                        if !client.name.contains(name) {
                            break;
                        }
                        valid = false;
                    }
                    SearchType::Exact(name) => {
                        if client.name != *name {
                            valid = false;
                        }
                    }
                    SearchType::None => {}
                };

                match &search.client {
                    SearchType::Fuzzy(name) => {
                        if !client.client.contains(name) {
                            break;
                        }
                        valid = false;
                    }
                    SearchType::Exact(name) => {
                        if client.client != *name {
                            valid = false;
                        }
                    }
                    SearchType::None => {}
                };

                match &search.other {
                    SearchType::Fuzzy(name) => {
                        let mut finds = 0usize;

                        for by in &client.other {
                            if let Some(b) = name.get(finds) {
                                if *by == *b {
                                    finds += 1;
                                } else {
                                    finds = 0;
                                }
                                if finds == client.other.len() {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        if name.len() < finds {
                            valid = false
                        }
                    }
                    SearchType::Exact(name) => {
                        if client.other != *name {
                            valid = false;
                        }
                    }
                    SearchType::None => {}
                };

                if valid {
                    adresses.push(client.adress.clone());
                }
            }

            let pak = Packets::SearchResponse(SearchResponse { session, adresses });
            let mut bytes = pak.to_bytes();
            bytes.reverse();

            let _ = self.clients.get_mut(index).unwrap().conn.send(&bytes);
        }

        for (index, info) in to_info {
            if let Some(client) = self.clients.get(index) {
                if client.session != info.session {
                    continue;
                }
            } else {
                continue;
            }

            let mut pak = Info {
                has: false,
                name: String::new(),
                client: String::new(),
                other: Vec::new(),
                adress: vec![],
            };

            for client in self.clients.iter() {
                if client.adress == info.adress {
                    pak.has = true;
                    pak.name = client.name.clone();
                    pak.client = client.client.clone();
                    pak.other = client.other.clone();
                    pak.adress = client.adress.clone();
                    break;
                }
            }

            if let Some(client) = self.clients.get_mut(index) {
                let pak = Packets::Info(pak);
                let mut bytes = pak.to_bytes();
                bytes.reverse();

                let _ = client.conn.send(&bytes);
            }
        }

        for (index, request) in to_request {
            let mut session = None;

            let mut from = None;
            if let Some(client) = self.clients.get(index) {
                from = Some(client.adress.clone());
            }
            let Some(from) = from else{continue};

            for client in self.clients.iter_mut() {
                if client.adress == request.to {
                    let pak = Packets::NewRequest(NewRequest {
                        session: client.session,
                        from,
                        secret: request.secret,
                    });
                    let mut bytes = pak.to_bytes();
                    bytes.reverse();
                    let _ = client.conn.send(&bytes);
                    session = Some(client.session);
                    break;
                }
            }

            if let Some(client) = self.clients.get_mut(index) {
                if let Some(session) = session {
                    client.to_connect.push(Connecting::Start(session))
                } else {
                    let pak = Packets::NewRequestResponse(NewRequestResponse {
                        session: client.session,
                        from: request.to,
                        accepted: false,
                        secret: String::new(),
                    });
                    let mut bytes = pak.to_bytes();
                    bytes.reverse();

                    let _ = client.conn.send(&bytes);
                }
            }
        }

        for (index, request_response) in to_request_response {
            let mut to = None;
            for client in self.clients.iter() {
                if client.adress == request_response.to {
                    for session in client.to_connect.iter() {
                        if session.session() == request_response.session {
                            to = Some(client.session);
                            break;
                        }
                    }
                }
            }

            let mut from = None;
            let mut uid = None;
            if let Some(conn) = self.clients.get(index) {
                from = Some(conn.adress.clone());
                uid = Some(conn.session);
            }

            let Some(from) = from else{continue};
            let Some(to) = to else{continue};
            let Some(uid) = uid else {continue};

            for client in self.clients.iter_mut() {
                if client.adress == request_response.to {
                    let pak = NewRequestResponse {
                        session: client.session,
                        from,
                        accepted: request_response.accepted,
                        secret: request_response.secret,
                    };
                    let mut bytes = Packets::NewRequestResponse(pak).to_bytes();
                    bytes.reverse();
                    let _ = client.conn.send(&bytes);
                    break;
                }
            }

            if request_response.accepted {
                if let Some(client) = self.clients.get_mut(index) {
                    client.to_connect.push(Connecting::Start(to))
                }
            } else {
                for client in self.clients.iter_mut() {
                    if client.session == to {
                        client.to_connect.retain(|to_conn| to_conn.session() != uid);
                        break;
                    }
                }
            }
        }

        for (index, request_final) in to_request_final {
            let mut to = None;
            for client in self.clients.iter() {
                if client.adress == request_final.to {
                    if client
                        .to_connect
                        .contains(&Connecting::Start(request_final.session))
                    {
                        to = Some(client.session)
                    }
                    break;
                }
            }

            let mut from = None;
            if let Some(conn) = self.clients.get(index) {
                from = Some(conn.adress.clone())
            }

            let Some(from) = from else{continue};
            let Some(to) = to else{continue};

            let mut session = None;
            for client in self.clients.iter_mut() {
                if client.adress == request_final.to {
                    let pak = NewRequestFinal {
                        session: client.session,
                        from,
                        accepted: request_final.accepted,
                    };
                    let mut bytes = Packets::NewRequestFinal(pak).to_bytes();
                    bytes.reverse();
                    let _ = client.conn.send(&bytes);
                    session = Some(client.session);
                    if request_final.accepted {
                        for to_conn in client.to_connect.iter_mut() {
                            if to_conn.session() == request_final.session {
                                *to_conn = Connecting::Finishing(to_conn.session());
                                break;
                            }
                        }
                    } else {
                        client
                            .to_connect
                            .retain(|to_conn| to_conn.session() != request_final.session);
                    }
                    break;
                }
            }

            let Some(session) = session else{continue};
            if let Some(client) = self.clients.get_mut(index) {
                if request_final.accepted {
                    for to_conn in client.to_connect.iter_mut() {
                        if to_conn.session() == session {
                            *to_conn = Connecting::Finishing(to_conn.session());
                            break;
                        }
                    }
                } else {
                    client
                        .to_connect
                        .retain(|to_conn| to_conn.session() != session);
                }
            }
        }
    }

    pub fn connect(&mut self) {
        let mut connect = Vec::new();

        for client in self.clients.iter() {
            if !client.to_connect.is_empty() {
                if !client.ports.is_empty() {
                    for to_conn in client.to_connect.iter() {
                        match to_conn {
                            Connecting::Finishing(session) => {
                                connect.push((client.session, *session));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        for conn in connect {
            let mut index1 = 0;
            let mut index2 = 0;

            let mut connecting_to = None;
            for (i, client) in self.clients.iter().enumerate() {
                if client.session == conn.0 {
                    for conn_to in client.to_connect.iter() {
                        if conn_to.session() == conn.1 {
                            connecting_to = Some(client.session);
                            break;
                        }
                    }
                    index1 = i;
                    break;
                }
            }

            let mut is_falid = false;

            if let Some(connecting_to) = connecting_to {
                for (i, client) in self.clients.iter().enumerate() {
                    if client.session == conn.1 {
                        for conn_to in client.to_connect.iter() {
                            if conn_to.session() == connecting_to {
                                is_falid = true;
                                index2 = i;
                                break;
                            }
                        }
                    }
                }
            } else {
                continue;
            }

            if !is_falid {
                continue;
            }

            let mut port1 = None;
            let mut port2 = None;
            let mut adress1 = None;
            let mut adress2 = None;
            let mut addr1 = None;
            let mut addr2 = None;

            for client in self.clients.iter_mut() {
                if client.session == conn.0 {
                    port1 = client.ports.pop();
                    adress1 = Some(client.from.clone());
                    addr1 = Some(client.adress.clone());
                } else if client.session == conn.1 {
                    port2 = client.ports.pop();
                    adress2 = Some(client.from.clone());
                    addr2 = Some(client.adress.clone());
                } else {
                    continue;
                }

                if port1.is_some() && port2.is_some() {
                    break;
                }
            }

            if let Some(port1) = port1 {
                if let Some(port2) = port2 {
                    if let Some(client) = self.clients.get_mut(index1) {
                        client
                            .to_connect
                            .retain(|to_conn| to_conn.session() != conn.1);
                    }
                    if let Some(client) = self.clients.get_mut(index2) {
                        client
                            .to_connect
                            .retain(|to_conn| to_conn.session() != conn.0);
                    }

                    let Some(adress1) = adress1 else{continue};
                    let Some(adress2) = adress2 else{continue};
                    let Some(addr1) = addr1 else{continue};
                    let Some(addr2) = addr2 else{continue};

                    let time = SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                        + 10000000000;

                    let pak = ConnectOn {
                        session: conn.0,
                        to: format!("{}:{}", adress2.as_socket().unwrap().ip(), port2),
                        port: port1,
                        adress: addr2,
                        time,
                    };

                    let mut bytes = Packets::ConnectOn(pak).to_bytes();
                    bytes.reverse();
                    if let Some(client) = self.clients.get_mut(index1) {
                        let _ = client.conn.send(&bytes);
                    }

                    let pak = ConnectOn {
                        session: conn.1,
                        to: format!("{}:{}", adress1.as_socket().unwrap().ip(), port1),
                        port: port2,
                        adress: addr1,
                        time,
                    };

                    let mut bytes = Packets::ConnectOn(pak).to_bytes();
                    bytes.reverse();
                    if let Some(client) = self.clients.get_mut(index2) {
                        let _ = client.conn.send(&bytes);
                    }
                } else {
                    if let Some(client) = self.clients.get_mut(index1) {
                        if client.session == conn.0 {
                            client.ports.push(port1);
                            break;
                        }
                    }
                }
            } else {
                if let Some(port2) = port2 {
                    if let Some(client) = self.clients.get_mut(index2) {
                        if client.session == conn.1 {
                            client.ports.push(port2);
                            break;
                        }
                    }
                }
            }
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
