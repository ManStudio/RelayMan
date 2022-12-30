use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct RequestResponse {
    pub session: usize,
    pub to: Adress,
    pub accepted: bool,
    pub secret: String,
}

#[derive(Bytes, Clone, Debug)]
pub struct NewRequestResponse {
    pub session: usize,
    pub from: Adress,
    pub accepted: bool,
    pub secret: String,
}
