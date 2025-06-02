use std::fmt::{Display, Formatter};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::error::ProtoError;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum EleType {
    Nil,
    BOOL,
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
    BOOL(bool),
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
            Ele::BOOL(x) => f.write_fmt(format_args!("{}", x)),
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
        Ele::Text(val) // Optimized: directly move the String
    }
}

impl From<bool> for Ele {
    fn from(item: bool) -> Self {
        Ele::BOOL(item)
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
    type Error = ProtoError;

    fn try_into(self) -> Result<i32, ProtoError> {
        match self {
            Ele::I32(x) => Ok(x),
            _ => Err(ProtoError::WrongElementType),
        }
    }
}

impl TryInto<i64> for Ele {
    type Error = ProtoError;

    fn try_into(self) -> Result<i64, ProtoError> {
        match self {
            Ele::I32(x) => Ok(x as i64),
            Ele::I64(x) => Ok(x),
            _ => Err(ProtoError::WrongElementType),
        }
    }
}

impl TryInto<f32> for Ele {
    type Error = ProtoError;

    fn try_into(self) -> Result<f32, ProtoError> {
        match self {
            Ele::F32(x) => Ok(x),
            Ele::F64(x) => Ok(x as f32),
            _ => Err(ProtoError::WrongElementType),
        }
    }
}

impl TryInto<f64> for Ele {
    type Error = ProtoError;

    fn try_into(self) -> Result<f64, ProtoError> {
        match self {
            Ele::F32(x) => Ok(x as f64),
            Ele::F64(x) => Ok(x),
            _ => Err(ProtoError::WrongElementType),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Seq {
    Nil,
    SeqBOOL(Vec<bool>),
    SeqI32(Vec<i32>),
    SeqI64(Vec<i64>),
    SeqF32(Vec<f32>),
    SeqF64(Vec<f64>),
    SeqText(Vec<String>),
    SeqDateTime(Vec<u64>),
}

impl Seq {
    pub fn len(&self) -> usize {
        match self {
            Seq::SeqBOOL(vec) => vec.len(),
            Seq::SeqI32(vec) => vec.len(),
            Seq::SeqI64(vec) => vec.len(),
            Seq::SeqF32(vec) => vec.len(),
            Seq::SeqF64(vec) => vec.len(),
            Seq::SeqText(vec) => vec.len(),
            Seq::SeqDateTime(vec) => vec.len(),
            Seq::Nil => 0,
        }
    }

    pub fn nbytes(&self) -> usize {
        match self {
            Seq::SeqBOOL(vec) => vec.len() * std::mem::size_of::<bool>(),
            Seq::SeqI32(vec) => vec.len() * std::mem::size_of::<i32>(),
            Seq::SeqI64(vec) => vec.len() * std::mem::size_of::<i64>(),
            Seq::SeqF32(vec) => vec.len() * std::mem::size_of::<f32>(),
            Seq::SeqF64(vec) => vec.len() * std::mem::size_of::<f64>(),
            Seq::SeqText(vec) => vec.iter().map(|x| x.len()).sum(),
            Seq::SeqDateTime(vec) => vec.len() * std::mem::size_of::<u64>(),
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
            Seq::SeqBOOL(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::SeqI32(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::SeqI64(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::SeqF32(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::SeqF64(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::SeqText(vec) => vec.get(idx).map(|x| x.to_string()),
            Seq::SeqDateTime(vec) => vec.get(idx).map(|x| {
                let datetime: DateTime<Utc> =
                    (SystemTime::UNIX_EPOCH + Duration::from_micros(*x)).into();
                datetime.to_rfc3339()
            }),
            Seq::Nil => None,
        }
    }

    pub fn get(&self, idx: usize) -> Ele {
        match self {
            Seq::SeqBOOL(vec) => vec.get(idx).map(|x| Ele::BOOL(*x)),
            Seq::SeqI32(vec) => vec.get(idx).map(|x| Ele::I32(*x)),
            Seq::SeqI64(vec) => vec.get(idx).map(|x| Ele::I64(*x)),
            Seq::SeqF32(vec) => vec.get(idx).map(|x| Ele::F32(*x)),
            Seq::SeqF64(vec) => vec.get(idx).map(|x| Ele::F64(*x)),
            Seq::SeqText(vec) => vec.get(idx).map(|x| Ele::Text(x.clone())),
            Seq::SeqDateTime(vec) => vec.get(idx).map(|x| Ele::DataTime(*x)),
            Seq::Nil => None,
        }
        .unwrap_or(Ele::Nil)
    }

    pub fn append(&mut self, value: impl Into<Ele>) -> Result<(), ProtoError> {
        let value = value.into();
        match (&mut *self, value) {
            (Seq::Nil, Ele::I32(x)) => *self = Seq::SeqI32(vec![x]),
            (Seq::Nil, Ele::I64(x)) => *self = Seq::SeqI64(vec![x]),
            (Seq::Nil, Ele::F32(x)) => *self = Seq::SeqF32(vec![x]),
            (Seq::Nil, Ele::F64(x)) => *self = Seq::SeqF64(vec![x]),
            (Seq::Nil, Ele::Text(x)) => *self = Seq::SeqText(vec![x]),
            (Seq::Nil, Ele::DataTime(x)) => *self = Seq::SeqDateTime(vec![x]),
            (Seq::Nil, Ele::Nil) => {} // Nil值不改变Nil序列
            (Seq::SeqI32(vec), Ele::I32(x)) => vec.push(x),
            (Seq::SeqI64(vec), Ele::I64(x)) => vec.push(x),
            (Seq::SeqF32(vec), Ele::F32(x)) => vec.push(x),
            (Seq::SeqF64(vec), Ele::F64(x)) => vec.push(x),
            (Seq::SeqText(vec), Ele::Text(x)) => vec.push(x),
            (Seq::SeqDateTime(vec), Ele::DataTime(x)) => vec.push(x),
            _ => return Err(ProtoError::WrongSequenceType),
        }
        Ok(())
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Value {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
    pub value: Option<String>,
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "value: {:?}", self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seq_append_from_nil() {
        let mut seq = Seq::Nil;

        // Test creating different sequence types from Nil
        assert!(seq.append(42i32).is_ok());
        assert_eq!(seq, Seq::SeqI32(vec![42]));

        let mut seq = Seq::Nil;
        assert!(seq.append("hello".to_string()).is_ok());
        assert_eq!(seq, Seq::SeqText(vec!["hello".to_string()]));

        let mut seq = Seq::Nil;
        assert!(seq.append(3.14f64).is_ok());
        assert_eq!(seq, Seq::SeqF64(vec![3.14]));
    }

    #[test]
    fn test_seq_append_type_mismatch() {
        let mut seq = Seq::SeqI32(vec![1, 2, 3]);

        // Should fail when trying to append wrong type
        assert!(seq.append("string".to_string()).is_err());
        assert_eq!(seq, Seq::SeqI32(vec![1, 2, 3])); // unchanged
    }

    #[test]
    fn test_seq_append_nil_to_nil() {
        let mut seq = Seq::Nil;
        assert!(seq.append(Ele::Nil).is_ok());
        assert_eq!(seq, Seq::Nil); // Should remain Nil
    }

    #[test]
    fn test_seq_len_and_empty() {
        let seq = Seq::Nil;
        assert_eq!(seq.len(), 0);
        assert!(seq.is_empty());

        let seq = Seq::SeqI32(vec![1, 2, 3]);
        assert_eq!(seq.len(), 3);
        assert!(!seq.is_empty());

        let seq = Seq::SeqText(vec![]);
        assert_eq!(seq.len(), 0);
        assert!(seq.is_empty());
    }
}
