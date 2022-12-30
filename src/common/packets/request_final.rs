use bytes_kman::prelude::*;

use crate::common::adress::Adress;

#[derive(Bytes, Clone, Debug)]
pub struct RequestFinal {
    pub session: usize,
    pub to: Adress,
    pub accepted: bool,
}

#[derive(Bytes, Clone, Debug)]
pub struct NewRequestFinal {
    pub session: usize,
    pub from: Adress,
    pub accepted: bool,
}
