use bytes_kman::prelude::*;

use crate::common::adress::Adress;

#[derive(Bytes, Clone, Debug)]
pub struct RequestFinal {
    pub session: u128,
    pub adress: Adress,
    pub accepted: bool,
}
