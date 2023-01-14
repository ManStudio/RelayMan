use bytes_kman::prelude::*;

use crate::common::adress::Adress;

#[derive(Bytes, Clone, Debug)]
pub struct Register {
    pub client: String,
    pub public: Adress,
    pub name: String,
    pub other: Vec<u8>,
    pub privacy: bool,
    pub private_adress: String,
}
