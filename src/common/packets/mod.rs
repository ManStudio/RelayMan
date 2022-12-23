use bytes_kman::prelude::*;

mod avalibile;
mod connect_on;
mod info;
mod info_request;
mod new_request;
mod register;
mod register_response;
mod request;
mod request_final;
mod request_response;
mod search;
mod search_response;
mod unregister;

pub use self::{
    avalibile::Avalibile, connect_on::ConnectOn, info::*, info_request::InfoRequest,
    new_request::NewRequest, register::Register, register_response::RegisterResponse,
    request::Request, request_final::RequestFinal, request_response::RequestResponse, search::*,
    search_response::SearchResponse, unregister::UnRegister,
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
    Avalibile(Avalibile),
    ConnectOn(ConnectOn),
    RequestFinal(RequestFinal),
    Tick { session: u128 },
}
