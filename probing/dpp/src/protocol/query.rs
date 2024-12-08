use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Query {
    pub expr: String,
    pub flags: Option<QueryFlags>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct QueryFlags {
    pub limit: Option<usize>,
    pub format: OutputFormat,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum OutputFormat{
    JSON,
    RON,
    BITCODE,
    #[default]
    ARROW,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct QueryResult {
    pub result: Vec<u8>,
    pub format: OutputFormat,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum QueryMessage {
    Query(Query),
    Result(QueryResult),
    
}