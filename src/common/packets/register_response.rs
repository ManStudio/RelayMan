use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct RegisterResponse {
    pub accepted: bool,
    pub session: u128,
}
