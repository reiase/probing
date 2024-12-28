use serde::{Deserialize, Serialize};

use crate::types::value::Value;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Table {
    pub names: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn new<N: Into<String>, V: Into<Value>>(names: Vec<N>, rows: Vec<Vec<V>>) -> Self {
        let names = names.into_iter().map(|x| x.into()).collect();
        let rows = rows
            .into_iter()
            .map(|r| r.into_iter().map(|x| x.into()).collect())
            .collect();
        Self { names, rows }
    }
}
