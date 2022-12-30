use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub struct UnRegister {
    pub session: usize,
}
