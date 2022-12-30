use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct Avalibile {
    pub session: usize,
    pub port: u16,
}
