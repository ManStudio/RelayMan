use bytes_kman::prelude::*;

use crate::common::adress::Adress;

#[derive(Bytes, Clone, Debug)]
pub struct Info {
    pub has: bool,
    pub name: String,
    pub client: String,
    pub other: Vec<u8>,
    pub adress: Adress,
}
