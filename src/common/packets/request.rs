use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct Request {
    pub session: usize,
    pub to: Adress,
    pub secret: String,
}

#[derive(Bytes, Clone, Debug)]
pub struct NewRequest {
    pub session: usize,
    pub from: Adress,
    pub secret: String,
}
