use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug, Default)]
pub enum SearchType<T> {
    Fuzzy(T),
    Exact(T),
    #[default]
    None,
}

#[derive(Bytes, Clone, Debug, Default)]
pub struct Search {
    pub session: usize,
    pub client: SearchType<String>,
    pub name: SearchType<String>,
    pub other: SearchType<Vec<u8>>,
}
