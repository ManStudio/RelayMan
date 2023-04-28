use std::{
    mem::MaybeUninit,
    net::{SocketAddr, ToSocketAddrs},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use crate::common::{adress::Adress, packets::Packets, FromRawSock, IntoRawSock, RawSock};

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
    pub fn add_socket(&self, socket: &Socket) -> RegisterResponse {
        self.connection.add_socket(socket)
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
    pub fn add_socket(&self, socket: &Socket) -> RegisterResponse {
        self.connection.add_socket(socket)
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

#[derive(Debug)]
pub enum RegisterResponse {
    Success { port: u16 },
    Error,
}

#[derive(Debug)]
pub enum ConnectOnError {
    CannotBind,
    CannotSetNonBlocking,
    TimoutIsLesTheResend,
    StageOneFailed,
    StageTwoFailed,
}

#[derive(Debug)]
pub struct Conn {
    pub port: u16,
    pub fd: RawSock,
    pub socket: Socket,
    pub addr: SocketAddr,
}

impl Conn {
    pub fn fd(&self) -> RawSock {
        self.fd
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn socket(&self) -> &Socket {
        &self.socket
    }

    pub fn mut_socket(&mut self) -> &mut Socket {
        &mut self.socket
    }
}

impl std::ops::Deref for Conn {
    type Target = Socket;

    fn deref(&self) -> &Self::Target {
        &self.socket
    }
}

impl std::ops::DerefMut for Conn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.socket
    }
}

impl ConnectOn {
    /// timeout need to be bigger then resend
    pub fn connect(
        self,
        timeout: Duration,
        resend: Duration,
        socket: Socket,
    ) -> Result<Conn, ConnectOnError> {
        if timeout < resend {
            return Err(ConnectOnError::TimoutIsLesTheResend);
        }

        let addr = self.to.to_socket_addrs().unwrap().next().unwrap();
        let sock_addr = SockAddr::from(addr);

        let fd = socket.into_raw();
        let mut conn = Conn {
            fd,
            port: self.port,
            socket: Socket::from_raw(fd),
            addr,
        };

        let Ok(_) = conn.set_nonblocking(true) else {return Err(ConnectOnError::CannotSetNonBlocking)};
        let _ = conn.set_read_timeout(Some(resend));
        let _ = conn.set_write_timeout(Some(resend));
        conn.set_ttl(3600);

        while SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            < self.time
        {}

        println!("Start");

        let time = SystemTime::now();
        let mut time_send = time;

        let mut buffer = [MaybeUninit::new(0); 4];
        let message = [1, 4, 21, 6];

        let _ = conn.send_to(&message, &sock_addr);

        loop {
            if time.elapsed().unwrap() > timeout {
                return Err(ConnectOnError::StageOneFailed);
            }

            if let Ok((len, from)) = conn.recv_from(&mut buffer) {
                if from.as_socket().unwrap() == sock_addr.as_socket().unwrap()
                    && unsafe { std::mem::transmute::<&[MaybeUninit<u8>], &[u8]>(&buffer[0..len]) }
                        == message
                {
                    conn.connect(&sock_addr).unwrap();
                    println!("First stage succesful!");
                    break;
                }
            }

            if time_send.elapsed().unwrap() > resend {
                time_send = SystemTime::now();
                let _ = conn.send_to(&message, &sock_addr);
            }
        }

        let message = [21, 20, 20, 21];
        let time = SystemTime::now();
        let mut time_send = time;
        let _ = conn.send(&message);

        loop {
            if time.elapsed().unwrap() > timeout {
                return Err(ConnectOnError::StageTwoFailed);
            }

            if let Ok(len) = conn.recv(&mut buffer) {
                let buffer =
                    unsafe { std::mem::transmute::<&[MaybeUninit<u8>], &[u8]>(&buffer[0..len]) };
                if buffer == message {
                    break;
                }
            }

            if time_send.elapsed().unwrap() > resend {
                time_send = SystemTime::now();
                let _ = conn.send(&message);
            }
        }

        Ok(conn)
    }
}

pub struct SearchResponse {
    pub adresses: Vec<Adress>,
}
