use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct NewRequest {
    pub session: u128,
    pub from: Adress,
    pub secret: String,
}
