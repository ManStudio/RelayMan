use bytes_kman::prelude::*;

use crate::common::adress::Adress;

#[derive(Bytes, Clone, Debug)]
pub struct ConnectOn {
    pub session: u128,
    pub to: String,
    pub adress: Adress,
    pub time: u128,
}
