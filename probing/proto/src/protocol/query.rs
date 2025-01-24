use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Options {
    pub limit: Option<usize>,
    pub format: Format,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Format {
    JSON,
    RON,
    BITCODE,
    #[default]
    ARROW,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum Message {
    #[default]
    Nil,
    Query {
        expr: String,
        opts: Option<Options>,
    },
    Reply {
        data: Vec<u8>,
        format: Format,
    },
    Error {
        message: String,
    },
}
