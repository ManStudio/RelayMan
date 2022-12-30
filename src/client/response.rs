use std::{
    mem::MaybeUninit,
    net::ToSocketAddrs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::common::{adress::Adress, packets::Packets};

use super::TConnection;

pub struct Response<T, R> {
    pub connection: T,
    pub packets: Packets,
    pub fn_has: fn(&T, &Packets) -> bool,
    pub fn_get: fn(T, Packets) -> R,
}

impl<T, R> Response<T, R> {
    pub fn has(&self) -> bool {
        (self.fn_has)(&self.connection, &self.packets)
    }

    pub fn get(self) -> R {
        while !self.has() {
            std::thread::sleep(Duration::from_millis(0));
        }
        (self.fn_get)(self.connection, self.packets)
    }
}

pub enum RequestStage {
    NewRequest(NewRequest),
    NewRequestResponse(NewRequestResponse),
    NewRequestFinal(NewRequestFinal),
    ConnectOn(ConnectOn),
}

pub struct NewRequest {
    pub connection: Box<dyn TConnection>,
    pub from: Adress,
    pub secret: String,
}

impl NewRequest {
    pub fn accept(self, accept: bool) -> Response<Box<dyn TConnection>, NewRequestFinal> {
        self.connection.request_response(&self.from, accept)
    }
}

pub struct NewRequestResponse {
    pub connection: Box<dyn TConnection>,
    pub from: Adress,
    pub accept: bool,
    pub secret: String,
}

impl NewRequestResponse {
    pub fn add_port(&self, port: u16) {
        self.connection.add_port(port)
    }

    /// `time_offset` should be in nanosecconds
    pub fn accept(
        self,
        accept: bool,
        time_offset: Option<u128>,
    ) -> Response<Box<dyn TConnection>, ConnectOn> {
        self.connection
            .request_final(&self.from, accept, time_offset)
    }
}

pub struct NewRequestFinal {
    pub connection: Box<dyn TConnection>,
    pub from: Adress,
    pub accept: bool,
}

impl NewRequestFinal {
    pub fn add_port(&self, port: u16) {
        self.connection.add_port(port)
    }
}

pub struct ConnectOn {
    pub connection: Box<dyn TConnection + Send>,
    pub adress: Adress,
    pub to: String,
    pub port: u16,
    pub time: u128,
}

impl std::fmt::Debug for ConnectOn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectOn")
            .field("adress", &self.adress)
            .field("to", &self.to)
            .field("port", &self.port)
            .field("time", &self.time)
            .finish()
    }
}

impl ConnectOn {
    /// timeout need to be bigger then resend
    pub fn connect(
        self,
        timeout: Duration,
        resend: Duration,
        nonblocking: bool,
    ) -> Result<Socket, ()> {
        if timeout < resend {
            return Err(());
        }

        let my_addr = format!("localhost:{}", self.port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();
        let addr = self.to.to_socket_addrs().unwrap().next().unwrap();

        let sock_my_addr = SockAddr::from(my_addr);
        let sock_addr = SockAddr::from(addr);

        let socket =
            Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP)).unwrap();
        let Ok(_) = socket.bind(&sock_my_addr) else{return Err(())};
        let Ok(_) = socket.set_nonblocking(nonblocking) else {return Err(())};
        let _ = socket.set_read_timeout(Some(timeout));
        let _ = socket.set_write_timeout(Some(timeout));

        while SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            < self.time
        {}

        let time = SystemTime::now();
        let mut time_send = time.clone();

        let mut buffer = [MaybeUninit::new(0); 4];
        let message = [1, 4, 21, 6];
        let _ = socket.send_to(&message, &sock_addr);

        loop {
            if time.elapsed().unwrap() > timeout {
                return Err(());
            }

            if time_send.elapsed().unwrap() > resend {
                time_send = SystemTime::now();
                let _ = socket.send(&message);
            }

            if let Ok((_, from)) = socket.recv_from(&mut buffer) {
                if from.as_socket().unwrap() == sock_addr.as_socket().unwrap() {
                    if unsafe { std::mem::transmute::<&[MaybeUninit<u8>], &[u8]>(&buffer) }
                        == message
                    {
                        socket.connect(&sock_addr).unwrap();
                        println!("First stage succesful!");
                        break;
                    }
                }
            }
        }

        let message = [21, 20, 20, 21];
        let time = SystemTime::now();
        let mut time_send = time.clone();
        let _ = socket.send(&message);

        loop {
            if time.elapsed().unwrap() > timeout {
                return Err(());
            }

            if time_send.elapsed().unwrap() > resend {
                time_send = SystemTime::now();
                let _ = socket.send(&message);
            }

            if let Ok(_) = socket.recv(&mut buffer) {
                let buffer = unsafe { std::mem::transmute::<&[MaybeUninit<u8>], &[u8]>(&buffer) };
                if buffer == message {
                    break;
                }
            }
        }

        Ok(socket)
    }
}

pub struct SearchResponse {
    pub adresses: Vec<Adress>,
}
