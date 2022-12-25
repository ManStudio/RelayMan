use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct RequestResponse {
    pub session: u128,
    pub to: Adress,
    pub accepted: bool,
    pub secret: String,
}

#[derive(Bytes, Clone, Debug)]
pub struct NewRequestResponse {
    pub session: u128,
    pub from: Adress,
    pub accepted: bool,
    pub secret: String,
}
