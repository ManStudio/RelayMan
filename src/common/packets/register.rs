use bytes_kman::prelude::*;

use crate::common::adress::Adress;

#[derive(Bytes, Clone, Debug)]
pub enum Register {
    Client {
        client: String,
        public: Adress,
        name: String,
        other: Vec<u8>,
        privacy: bool,
        private_adress: String,
    },
    Port {
        session: usize,
    },
}
