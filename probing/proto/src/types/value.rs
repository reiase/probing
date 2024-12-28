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
