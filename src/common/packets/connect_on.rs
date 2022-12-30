use bytes_kman::prelude::*;

use crate::common::adress::Adress;

#[derive(Bytes, Clone, Debug)]
pub struct ConnectOn {
    pub session: usize,
    pub to: String,
    pub port: u16,
    pub adress: Adress,
    pub time: u128,
}
