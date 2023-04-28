use bytes_kman::prelude::*;

mod connect_on;
mod info;
mod info_request;
mod register;
mod register_response;
mod request;
mod request_final;
mod request_response;
mod search;
mod search_response;
mod unregister;

pub use self::{
    connect_on::*, info::*, info_request::*, register::*, register_response::*, request::*,
    request_final::*, request_response::*, search::*, search_response::*, unregister::*,
};

#[derive(Bytes, Clone, Debug)]
pub enum Packets {
    Register(Register),
    RegisterResponse(RegisterResponse),
    UnRegister(UnRegister),
    Search(Search),
    SearchResponse(SearchResponse),
    Info(Info),
    InfoRequest(InfoRequest),
    Request(Request),
    NewRequest(NewRequest),
    RequestResponse(RequestResponse),
    NewRequestResponse(NewRequestResponse),
    RequestFinal(RequestFinal),
    NewRequestFinal(NewRequestFinal),
    ConnectOn(ConnectOn),
    Tick { session: usize },
}
