pub mod adress;
pub mod packets;

#[cfg(target_os = "windows")]
pub type RawSock = std::os::windows::io::RawSocket;
#[cfg(target_os = "linux")]
pub type RawSock = std::os::fd::RawFd;

pub trait FromRawSock {
    fn from_raw(raw_sock: RawSock) -> Self;
}

pub trait IntoRawSock {
    fn into_raw(self) -> RawSock;
}

impl FromRawSock for socket2::Socket {
    fn from_raw(raw_sock: RawSock) -> Self {
        #[cfg(target_os = "windows")]
        use std::os::windows::io::FromRawSocket;

        #[cfg(target_os = "windows")]
        unsafe {
            Self::from_raw_socket(raw_sock)
        }

        #[cfg(target_os = "linux")]
        use std::os::fd::FromRawFd;
        #[cfg(target_os = "linux")]
        unsafe {
            Self::from_raw_fd(raw_sock)
        }
    }
}

impl IntoRawSock for socket2::Socket {
    fn into_raw(self) -> RawSock {
        #[cfg(target_os = "windows")]
        use std::os::windows::io::IntoRawSocket;

        #[cfg(target_os = "windows")]
        unsafe {
            self.into_raw_socket()
        }

        #[cfg(target_os = "linux")]
        use std::os::fd::IntoRawFd;
        #[cfg(target_os = "linux")]
        unsafe {
            self.into_raw_fd()
        }
    }
}
