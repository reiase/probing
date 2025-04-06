use serde::{Deserialize, Serialize};

use crate::types::{DataFrame, TimeSeries};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Options {
    pub limit: Option<usize>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Query {
    pub expr: String,
    pub opts: Option<Options>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub enum Data {
    #[default]
    Nil,
    Error(String),
    DataFrame(DataFrame),
    TimeSeries(TimeSeries),
}
