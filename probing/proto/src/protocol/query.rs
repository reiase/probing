use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Query {
    pub expr: String,
    pub opts: Option<Options>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Options {
    pub limit: Option<usize>,
    pub format: Format,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Format{
    JSON,
    RON,
    BITCODE,
    #[default]
    ARROW,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Reply {
    pub data: Vec<u8>,
    pub format: Format,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Message {
    Query(Query),
    Reply(Reply),
}