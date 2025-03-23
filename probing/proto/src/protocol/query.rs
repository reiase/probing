use serde::{Deserialize, Serialize};

use crate::types::{DataFrame, TimeSeries};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Options {
    pub limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub enum Data {
    #[default]
    Nil,
    DataFrame(DataFrame),
    TimeSeries(TimeSeries),
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub enum Message {
    #[default]
    Nil,
    Query {
        expr: String,
        opts: Option<Options>,
    },
    Reply {
        data: Data,
    },
    Error {
        message: String,
    },
}
