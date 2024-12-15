use std::fmt::Display;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Value {
    Nil,
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Text(String),
    Url(String),
    DataTime(u64),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => f.write_str("nil"),
            Value::Int32(x) => f.write_fmt(format_args!("{x}")),
            Value::Int64(x) => f.write_fmt(format_args!("{x}")),
            Value::Float32(x) => f.write_fmt(format_args!("{x}")),
            Value::Float64(x) => f.write_fmt(format_args!("{x}")),
            Value::Text(x) => f.write_fmt(format_args!("{x}")),
            Value::Url(x) => f.write_fmt(format_args!("{x}")),
            Value::DataTime(x) => {
                let datetime: DateTime<Utc> =
                    (SystemTime::UNIX_EPOCH + Duration::from_micros(*x)).into();
                f.write_fmt(format_args!("{}", datetime.to_rfc3339()))
            }
        }
    }
}

impl From<&str> for Value {
    fn from(val: &str) -> Self {
        Value::Text(val.to_string())
    }
}

impl From<String> for Value {
    fn from(val: String) -> Self {
        Value::Text(val.to_string())
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Array {
    Nil,
    Int32Array(Vec<i32>),
    Int64Array(Vec<i64>),
    Float32Array(Vec<f32>),
    Float64Array(Vec<f64>),
    TextArray(Vec<String>),
    DateTimeArray(Vec<u64>),
}

impl Array {
    pub fn len(&self) -> usize {
        match self {
            Array::Int32Array(vec) => vec.len(),
            Array::Int64Array(vec) => vec.len(),
            Array::Float32Array(vec) => vec.len(),
            Array::Float64Array(vec) => vec.len(),
            Array::TextArray(vec) => vec.len(),
            Array::DateTimeArray(vec) => vec.len(),
            Array::Nil => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Array::Nil => true,
            other => other.len() == 0,
        }
    }

    pub fn get_str(&self, idx: usize) -> Option<String> {
        match self {
            Array::Int32Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::Int64Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::Float32Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::Float64Array(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::TextArray(vec) => vec.get(idx).map(|x| x.to_string()),
            Array::DateTimeArray(vec) => vec.get(idx).map(|x| {
                let datetime: DateTime<Utc> =
                    (SystemTime::UNIX_EPOCH + Duration::from_micros(*x)).into();
                datetime.to_rfc3339()
            }),
            Array::Nil => None,
        }
    }

    pub fn get(&self, idx: usize) -> Value {
        match self {
            Array::Int32Array(vec) => vec.get(idx).map(|x| Value::Int32(*x)),
            Array::Int64Array(vec) => vec.get(idx).map(|x| Value::Int64(*x)),
            Array::Float32Array(vec) => vec.get(idx).map(|x| Value::Float32(*x)),
            Array::Float64Array(vec) => vec.get(idx).map(|x| Value::Float64(*x)),
            Array::TextArray(vec) => vec.get(idx).map(|x| Value::Text(x.clone())),
            Array::DateTimeArray(vec) => vec.get(idx).map(|x| Value::DataTime(*x)),
            Array::Nil => None,
        }
        .unwrap_or(Value::Nil)
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
        let rows = rows
            .into_iter()
            .map(|r| r.into_iter().map(|x| x.into()).collect())
            .collect();
        Self { names, rows }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct DataFrame {
    pub names: Vec<String>,
    pub cols: Vec<Array>,
}

impl DataFrame {
    pub fn new(names: Vec<String>, columns: Vec<Array>) -> Self {
        DataFrame {
            names,
            cols: columns,
        }
    }
}
