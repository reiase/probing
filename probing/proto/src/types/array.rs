use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::value::Value;

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
