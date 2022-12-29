use bytes_kman::TBytes;

use crate::common::packets::{NewRequest, NewRequestResponse, Packets, Request};

use super::{Connecting, RelayServer};

impl RelayServer {
    pub(crate) fn on_request(&mut self, index: usize, request: Request) {
        let mut session = None;

        let mut from = None;
        if let Some(client) = self.clients.get(index) {
            from = Some(client.adress.clone());
        }
        let Some(from) = from else{return};

        for client in self.clients.iter_mut() {
            if client.adress == request.to {
                let pak = Packets::NewRequest(NewRequest {
                    session: client.session,
                    from,
                    secret: request.secret,
                });
                let mut bytes = pak.to_bytes();
                bytes.reverse();
                let _ = client.conn.send(&bytes);
                session = Some(client.session);
                break;
            }
        }

        if let Some(client) = self.clients.get_mut(index) {
            if let Some(session) = session {
                client.to_connect.push(Connecting::Start(session))
            } else {
                let pak = Packets::NewRequestResponse(NewRequestResponse {
                    session: client.session,
                    from: request.to,
                    accepted: false,
                    secret: String::new(),
                });
                let mut bytes = pak.to_bytes();
                bytes.reverse();

                let _ = client.conn.send(&bytes);
            }
        }
    }
}
