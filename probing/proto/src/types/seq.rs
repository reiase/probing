use std::time::{Duration, SystemTime};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::basic::Ele;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Seq {
    Nil,
    Int32Seq(Vec<i32>),
    Int64Seq(Vec<i64>),
    Float32Seq(Vec<f32>),
    Float64Seq(Vec<f64>),
    TextSeq(Vec<String>),
    DateTimeSeq(Vec<u64>),
}

impl Seq {
    pub fn len(&self) -> usize {
        match self {
            Seq::Int32Seq(vec) => vec.len(),
            Seq::Int64Seq(vec) => vec.len(),
            Seq::Float32Seq(vec) => vec.len(),
            Seq::Float64Seq(vec) => vec.len(),
            Seq::TextSeq(vec) => vec.len(),
            Seq::DateTimeSeq(vec) => vec.len(),
            Seq::Nil => 0,
        }
    }

    pub fn nbytes(&self) -> usize {
        match self {
            Seq::Int32Seq(vec) => vec.len() * std::mem::size_of::<i32>(),
            Seq::Int64Seq(vec) => vec.len() * std::mem::size_of::<i64>(),
            Seq::Float32Seq(vec) => vec.len() * std::mem::size_of::<f32>(),
            Seq::Float64Seq(vec) => vec.len() * std::mem::size_of::<f64>(),
            Seq::TextSeq(vec) => vec.iter().map(|x| x.len()).sum(),
            Seq::DateTimeSeq(vec) => vec.len() * std::mem::size_of::<u64>(),
            Seq::Nil => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Seq::Nil => true,
            other => other.len() == 0,
        }
    }

    pub fn get_str(&self, idx: usize) -> Option<String> {
        match self {
            Seq::Int32Seq(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::Int64Seq(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::Float32Seq(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::Float64Seq(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::TextSeq(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::DateTimeSeq(vec) => vec.get(idx).map(|x| {
                let datetime: DateTime<Utc> =
                    (SystemTime::UNIX_EPOCH + Duration::from_micros(*x)).into();
                datetime.to_rfc3339()
            }),
            Seq::Nil => None,
        }
    }

    pub fn get(&self, idx: usize) -> Ele {
        match self {
            Seq::Int32Seq(vec) => vec.get(idx).map(|x| Ele::I32(*x)),
            Seq::Int64Seq(vec) => vec.get(idx).map(|x| Ele::I64(*x)),
            Seq::Float32Seq(vec) => vec.get(idx).map(|x| Ele::F32(*x)),
            Seq::Float64Seq(vec) => vec.get(idx).map(|x| Ele::F64(*x)),
            Seq::TextSeq(vec) => vec.get(idx).map(|x| Ele::Text(x.clone())),
            Seq::DateTimeSeq(vec) => vec.get(idx).map(|x| Ele::DataTime(*x)),
            Seq::Nil => None,
        }
        .unwrap_or(Ele::Nil)
    }

    pub fn append(&mut self, value: impl Into<Ele>) -> Result<()> {
        let value = value.into();
        match (self, value) {
            (Seq::Nil, _value) => {}
            (Seq::Int32Seq(vec), Ele::I32(x)) => vec.push(x),
            (Seq::Int64Seq(vec), Ele::I64(x)) => vec.push(x),
            (Seq::Float32Seq(vec), Ele::F32(x)) => vec.push(x),
            (Seq::Float64Seq(vec), Ele::F64(x)) => vec.push(x),
            (Seq::TextSeq(vec), Ele::Text(x)) => vec.push(x),
            (Seq::DateTimeSeq(vec), Ele::DataTime(x)) => vec.push(x),
            _ => return Err(anyhow::anyhow!("Type mismatch")),
        }
        Ok(())
    }
}
