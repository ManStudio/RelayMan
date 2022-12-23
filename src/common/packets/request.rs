use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct Request {
    pub session: u128,
    pub to: Adress,
    pub secret: String,
}
