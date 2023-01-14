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
    my_adress: SocketAddr,
    addr: SocketAddr,
    port: u16,
    fd: RawSock,
    pub socket: Socket,
}

impl Conn {
    pub fn fd(&self) -> RawSock {
        self.fd
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn my_addr(&self) -> SocketAddr {
        self.my_adress
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

impl Drop for Conn {
    fn drop(&mut self) {
        if let Ok(upnp_gateway) = igd::search_gateway(igd::SearchOptions::default()) {
            if let SocketAddr::V4(_) = self.my_adress {
                let _ = upnp_gateway.remove_port(igd::PortMappingProtocol::UDP, self.port);
            }
        }
    }
}

impl ConnectOn {
    /// timeout need to be bigger then resend
    pub fn connect(
        self,
        timeout: Duration,
        resend: Duration,
        nonblocking: bool,
    ) -> Result<Conn, ConnectOnError> {
        if timeout < resend {
            return Err(ConnectOnError::TimoutIsLesTheResend);
        }

        let local_adress = local_ip_address::local_ip().unwrap();
        let addr = self.to.to_socket_addrs().unwrap().next().unwrap();
        let my_addr = format!("{}:{}", local_adress, self.port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();

        'try_to_port_forword: {
            let mut n = natpmp::Natpmp::new().unwrap();
            n.send_port_mapping_request(natpmp::Protocol::UDP, self.port, self.port, 30)
                .unwrap();

            loop {
                match n.read_response_or_retry() {
                    Ok(ok) => match ok {
                        natpmp::Response::UDP(res) => {
                            println!(
                                "PMP Open: public: {}, private: {}, lifetime: {:?}",
                                res.public_port(),
                                res.private_port(),
                                res.lifetime()
                            );
                            break 'try_to_port_forword;
                        }
                        _ => {
                            println!("PMP Failed: {:?}", ok);
                            break;
                        }
                    },
                    Err(err) => {
                        std::thread::sleep(Duration::from_millis(500));
                        match err {
                            natpmp::Error::NATPMP_TRYAGAIN => {}
                            _ => {
                                println!("PMP Error: {:?}", err);
                                break;
                            }
                        }
                    }
                }
            }

            if let Ok(upnp_gateway) = igd::search_gateway(igd::SearchOptions::default()) {
                if let SocketAddr::V4(ip) = my_addr {
                    let res = upnp_gateway.add_port(
                        igd::PortMappingProtocol::UDP,
                        self.port,
                        ip,
                        30,
                        "RelayMan",
                    );

                    match res {
                        Ok(_) => {
                            println!("UPNP Open port: {} for: {}", self.port, my_addr);
                            break 'try_to_port_forword;
                        }
                        Err(err) => println!("UPNP Error: {:?}", err),
                    }
                }
            } else {
                println!("No UPNP gateway");
            }
        }

        let sock_my_addr = SockAddr::from(my_addr);
        let sock_addr = SockAddr::from(addr);

        println!("SockAddr: {}", sock_addr.as_socket().unwrap());

        let socket = Socket::new(
            Domain::for_address(addr),
            Type::DGRAM,
            Some(Protocol::from(0)),
        )
        .unwrap();
        let fd = socket.into_raw();
        let conn = Conn {
            my_adress: my_addr,
            addr,
            fd,
            port: self.port,
            socket: Socket::from_raw(fd),
        };
        let Ok(_) = conn.bind(&sock_my_addr) else{return Err(ConnectOnError::CannotBind)};
        let Ok(_) = conn.set_nonblocking(nonblocking) else {return Err(ConnectOnError::CannotSetNonBlocking)};
        let _ = conn.set_read_timeout(Some(resend));
        let _ = conn.set_write_timeout(Some(resend));

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

        // conn.connect(&sock_addr).unwrap();
        // conn.send(&message).unwrap();
        let _ = conn.send_to(&message, &sock_addr);

        loop {
            if time.elapsed().unwrap() > timeout {
                return Err(ConnectOnError::StageOneFailed);
            }

            if let Ok((len, from)) = conn.recv_from(&mut buffer) {
                println!("Recb {:?}", from);
                if from.as_socket().unwrap() == sock_addr.as_socket().unwrap() {
                    if unsafe { std::mem::transmute::<&[MaybeUninit<u8>], &[u8]>(&buffer[0..len]) }
                        == message
                    {
                        conn.connect(&sock_addr).unwrap();
                        println!("First stage succesful!");
                        break;
                    }
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
