use bytes_kman::TBytes;

use crate::common::packets::{Packets, Search, SearchResponse, SearchType};

use super::RelayServer;

impl RelayServer {
    pub(crate) fn on_search(&mut self, index: usize, search: Search) {
        let session;
        if let Some(client) = self.clients.get(index) {
            session = client.session;
        } else {
            return;
        }

        let mut adresses = Vec::new();

        for client in self.clients.iter() {
            let mut valid = true;

            match &search.name {
                SearchType::Fuzzy(name) => {
                    if !client.name.contains(name) {
                        break;
                    }
                    valid = false;
                }
                SearchType::Exact(name) => {
                    if client.name != *name {
                        valid = false;
                    }
                }
                SearchType::None => {}
            };

            match &search.client {
                SearchType::Fuzzy(name) => {
                    if !client.client.contains(name) {
                        break;
                    }
                    valid = false;
                }
                SearchType::Exact(name) => {
                    if client.client != *name {
                        valid = false;
                    }
                }
                SearchType::None => {}
            };

            match &search.other {
                SearchType::Fuzzy(name) => {
                    let mut finds = 0usize;

                    for by in &client.other {
                        if let Some(b) = name.get(finds) {
                            if *by == *b {
                                finds += 1;
                            } else {
                                finds = 0;
                            }
                            if finds == client.other.len() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if name.len() < finds {
                        valid = false
                    }
                }
                SearchType::Exact(name) => {
                    if client.other != *name {
                        valid = false;
                    }
                }
                SearchType::None => {}
            };

            if valid {
                adresses.push(client.adress.clone());
            }
        }

        let pak = Packets::SearchResponse(SearchResponse { session, adresses });
        let mut bytes = pak.to_bytes();
        bytes.reverse();

        let _ = self.clients.get_mut(index).unwrap().conn.send(&bytes);
    }
}
