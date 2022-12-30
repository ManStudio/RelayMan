use crate::common::adress::Adress;
use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct SearchResponse {
    pub session: usize,
    pub adresses: Vec<Adress>,
}
