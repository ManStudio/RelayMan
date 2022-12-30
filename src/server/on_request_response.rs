use bytes_kman::TBytes;

use crate::common::packets::{NewRequestResponse, Packets, RequestResponse};

use super::{ClientStage, Connecting, RelayServer};

impl RelayServer {
    pub(crate) fn on_request_response(&mut self, index: usize, request_response: RequestResponse) {
        let mut to = None;
        for client in self.clients.iter() {
            if let ClientStage::Registered(rclient) = &client.stage {
                if rclient.adress == request_response.to {
                    for session in rclient.to_connect.iter() {
                        if session.session() == request_response.session {
                            to = Some(client.session);
                            break;
                        }
                    }
                }
            } else {
                return;
            }
        }

        let mut from = None;
        let mut uid = None;
        if let Some(client) = self.clients.get(index) {
            if let ClientStage::Registered(rclient) = &client.stage {
                from = Some(rclient.adress.clone());
                uid = Some(client.session);
            } else {
                return;
            }
        }

        let Some(from) = from else {return};
        let Some(to) = to else {return};
        let Some(uid) = uid else {return};

        for client in self.clients.iter_mut() {
            if let ClientStage::Registered(rclient) = &mut client.stage {
                if rclient.adress == request_response.to {
                    let pak = NewRequestResponse {
                        session: client.session,
                        from,
                        accepted: request_response.accepted,
                        secret: request_response.secret,
                    };
                    let mut bytes = Packets::NewRequestResponse(pak).to_bytes();
                    bytes.reverse();
                    let _ = client.conn.send(&bytes);
                    break;
                }
            }
        }

        if request_response.accepted {
            if let Some(client) = self.clients.get_mut(index) {
                if let ClientStage::Registered(rclient) = &mut client.stage {
                    rclient.to_connect.push(Connecting::Start(to))
                }
            }
        } else {
            for client in self.clients.iter_mut() {
                if let ClientStage::Registered(rclient) = &mut client.stage {
                    if client.session == to {
                        rclient
                            .to_connect
                            .retain(|to_conn| to_conn.session() != uid);
                        break;
                    }
                }
            }
        }
    }
}
