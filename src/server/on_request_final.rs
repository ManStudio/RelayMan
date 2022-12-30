use bytes_kman::TBytes;

use crate::common::packets::{NewRequestFinal, Packets, RequestFinal};

use super::{ClientStage, Connecting, RelayServer};

impl RelayServer {
    pub(crate) fn on_request_final(&mut self, index: usize, request_final: RequestFinal) {
        let mut to = None;
        for client in self.clients.iter() {
            if let ClientStage::Registered(rclient) = &client.stage {
                if rclient.adress == request_final.to {
                    if rclient
                        .to_connect
                        .contains(&Connecting::Start(request_final.session))
                    {
                        to = Some(client.session)
                    }
                    break;
                }
            } else {
                return;
            }
        }

        let mut from = None;
        if let Some(client) = self.clients.get(index) {
            if let ClientStage::Registered(rclient) = &client.stage {
                from = Some(rclient.adress.clone())
            } else {
                return;
            }
        }

        let Some(from) = from else{return};
        let Some(_) = to else{return};

        let mut session = None;
        for client in self.clients.iter_mut() {
            if let ClientStage::Registered(rclient) = &mut client.stage {
                if rclient.adress == request_final.to {
                    let pak = NewRequestFinal {
                        session: client.session,
                        from,
                        accepted: request_final.accepted,
                    };
                    let mut bytes = Packets::NewRequestFinal(pak).to_bytes();
                    bytes.reverse();
                    let _ = client.conn.send(&bytes);
                    session = Some(client.session);
                    if request_final.accepted {
                        for to_conn in rclient.to_connect.iter_mut() {
                            if to_conn.session() == request_final.session {
                                *to_conn = Connecting::Finishing(to_conn.session());
                                break;
                            }
                        }
                    } else {
                        rclient
                            .to_connect
                            .retain(|to_conn| to_conn.session() != request_final.session);
                    }
                    break;
                }
            } else {
                return;
            }
        }

        let Some(session) = session else{return};
        if let Some(client) = self.clients.get_mut(index) {
            if let ClientStage::Registered(rclient) = &mut client.stage {
                if request_final.accepted {
                    for to_conn in rclient.to_connect.iter_mut() {
                        if to_conn.session() == session {
                            *to_conn = Connecting::Finishing(to_conn.session());
                            break;
                        }
                    }
                } else {
                    rclient
                        .to_connect
                        .retain(|to_conn| to_conn.session() != session);
                }
            }
        }
    }
}
