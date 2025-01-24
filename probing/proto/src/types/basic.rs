use std::fmt::Display;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum EleType {
    Nil,
    I32,
    I64,
    F32,
    F64,
    Text,
    Url,
    DataTime,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Ele {
    Nil,
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Text(String),
    Url(String),
    DataTime(u64),
}

impl Display for Ele {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ele::Nil => f.write_str("nil"),
            Ele::I32(x) => f.write_fmt(format_args!("{x}")),
            Ele::I64(x) => f.write_fmt(format_args!("{x}")),
            Ele::F32(x) => f.write_fmt(format_args!("{x}")),
            Ele::F64(x) => f.write_fmt(format_args!("{x}")),
            Ele::Text(x) => f.write_fmt(format_args!("{x}")),
            Ele::Url(x) => f.write_fmt(format_args!("{x}")),
            Ele::DataTime(x) => {
                let datetime: DateTime<Utc> =
                    (SystemTime::UNIX_EPOCH + Duration::from_micros(*x)).into();
                f.write_fmt(format_args!("{}", datetime.to_rfc3339()))
            }
        }
    }
}

impl From<&str> for Ele {
    fn from(val: &str) -> Self {
        Ele::Text(val.to_string())
    }
}

impl From<String> for Ele {
    fn from(val: String) -> Self {
        Ele::Text(val.to_string())
    }
}

impl From<i32> for Ele {
    fn from(item: i32) -> Self {
        Ele::I32(item)
    }
}

impl From<i64> for Ele {
    fn from(item: i64) -> Self {
        Ele::I64(item)
    }
}

impl From<f32> for Ele {
    fn from(item: f32) -> Self {
        Ele::F32(item)
    }
}

impl From<f64> for Ele {
    fn from(item: f64) -> Self {
        Ele::F64(item)
    }
}

impl TryInto<i32> for Ele {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<i32> {
        match self {
            Ele::I32(x) => Ok(x),
            _ => anyhow::bail!("Value is not an i32"),
        }
    }
}

impl TryInto<i64> for Ele {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<i64> {
        match self {
            Ele::I32(x) => Ok(x as i64),
            Ele::I64(x) => Ok(x),
            _ => anyhow::bail!("Value is not an i64"),
        }
    }
}
