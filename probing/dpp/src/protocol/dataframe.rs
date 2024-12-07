use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Value {
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Text(String),
    Url(String),
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::Int32(x) => x.to_string(),
            Value::Int64(x) => x.to_string(),
            Value::Float32(x) => x.to_string(),
            Value::Float64(x) => x.to_string(),
            Value::Text(x) => x.to_string(),
            Value::Url(x) => x.to_string(),
        }
    }
}

impl Into<Value> for &str {
    fn into(self) -> Value {
        Value::Text(self.to_string())
    }
}

impl Into<Value> for String {
    fn into(self) -> Value {
        Value::Text(self.to_string())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Array {
    Int32Array(Vec<i32>),
    Int64Array(Vec<i64>),
    Float32Array(Vec<f32>),
    Float64Array(Vec<f64>),
    TextArray(Vec<String>),
}

impl Array {
    pub fn len(&self) -> usize {
        match self {
            Array::Int32Array(vec) => vec.len(),
            Array::Int64Array(vec) => vec.len(),
            Array::Float32Array(vec) => vec.len(),
            Array::Float64Array(vec) => vec.len(),
            Array::TextArray(vec) => vec.len(),
        }
    }

    pub fn get_str(&self, idx: usize) -> Option<String> {
        match self {
            Array::Int32Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::Int64Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::Float32Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::Float64Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::TextArray(vec) => vec.get(idx).map(|x| x.to_string()),
        }
    }

    pub fn get(&self, idx: usize) -> Value {
        self.get(idx)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Table {
    pub names: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn new<N: Into<String>, V: Into<Value>>(names: Vec<N>, rows: Vec<Vec<V>>) -> Self {
        let names = names.into_iter().map(|x| x.into()).collect();
        let rows = rows.into_iter().map(|r| {
            r.into_iter().map(|x| x.into()).collect()
        }).collect();
        Self { names, rows }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct DataFrame {
    pub names: Vec<String>,
    pub cols: Vec<Array>,
}
