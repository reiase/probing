use std::fmt::Display;

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

impl Query {
    pub fn new(expr: String) -> Self {
        Self { expr, opts: None }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub enum Data {
    #[default]
    Nil,
    Error(QueryError),
    DataFrame(DataFrame),
    TimeSeries(TimeSeries),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QueryError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ErrorCode {
    ParseError,
    ExecutionError,
    TimeoutError,
    ResourceExhausted,
    PermissionDenied,
    NotFound,
    Internal,
}

impl Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QueryError: {:?} - {}", self.code, self.message)
    }
}
