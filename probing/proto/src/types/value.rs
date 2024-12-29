use std::fmt::Display;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub enum DataType {
    Nil,
    Int32,
    Int64,
    Float32,
    Float64,
    Text,
    Url,
    DataTime,
}

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

impl From<i32> for Value {
    fn from(item: i32) -> Self {
        Value::Int32(item)
    }
}

impl From<i64> for Value {
    fn from(item: i64) -> Self {
        Value::Int64(item)
    }
}

impl TryInto<i32> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<i32> {
        match self {
            Value::Int32(x) => Ok(x),
            _ => anyhow::bail!("Value is not an i32"),
            
        }
    }
}

impl TryInto<i64> for Value {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<i64> {
        match self {
            Value::Int32(x) => Ok(x as i64),
            Value::Int64(x) => Ok(x),
            _ => anyhow::bail!("Value is not an i64"),
            
        }
    }
}