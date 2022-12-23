use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct InfoRequest {
    pub adress: Adress,
    pub session: u128,
}
