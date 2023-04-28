use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub enum RegisterResponse {
    Client { accepted: bool, session: usize },
    Port { port: u16 },
}

impl RegisterResponse {
    pub fn accepted(&self) -> bool {
        match self {
            RegisterResponse::Client { accepted, session } => *accepted,
            RegisterResponse::Port { port } => true,
        }
    }
}
