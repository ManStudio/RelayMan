use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct Avalibile {
    pub session: u128,
    pub port: u16,
}
