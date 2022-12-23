use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct SearchResponse {
    pub session: u128,
    pub adresses: Vec<Adress>,
}
