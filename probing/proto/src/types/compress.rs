use std::collections::BTreeMap;

use pco::data_types::Number;
use pco::standalone::{simple_decompress, simpler_compress};

use super::{EleType, ProtoError, Seq};

pub type CodeBook = Option<BTreeMap<i64, String>>;

pub trait Compressable {
    fn compress(&self) -> Result<(EleType, Vec<u8>, CodeBook), ProtoError>;
}

pub trait Decompressable
where
    Self: Sized,
{
    fn decompress(dtype: EleType, data: &[u8], cb: &CodeBook) -> Result<Self, ProtoError>;
}

impl Compressable for Seq {
    fn compress(&self) -> Result<(EleType, Vec<u8>, CodeBook), ProtoError> {
        match self {
            Seq::Nil => Ok((EleType::Nil, Default::default(), None)),
            Seq::SeqI32(vec) => Ok((EleType::I32, sample_compress(vec)?, None)),
            Seq::SeqI64(vec) => Ok((EleType::I64, sample_compress(vec)?, None)),
            Seq::SeqF32(vec) => Ok((EleType::F32, sample_compress(vec)?, None)),
            Seq::SeqF64(vec) => Ok((EleType::F64, sample_compress(vec)?, None)),
            Seq::SeqText(vec) => {
                let (data, cb) = text_compress(vec)?;
                Ok((EleType::Text, data, cb))
            }
            Seq::SeqDateTime(vec) => Ok((EleType::DataTime, sample_compress(vec)?, None)),
        }
    }
}

fn sample_compress<T: Number>(data: &[T]) -> Result<Vec<u8>, ProtoError> {
    let compressed = simpler_compress(data, 0);
    match compressed {
        Ok(mut compressed) => {
            compressed.shrink_to_fit();
            Ok(compressed)
        }
        Err(err) => Err(ProtoError::CompressError(err.to_string())),
    }
}

fn text_compress(data: &Vec<String>) -> Result<(Vec<u8>, CodeBook), ProtoError> {
    let mut cb: BTreeMap<String, i64> = Default::default();
    let mut compressed: Vec<i64> = Vec::with_capacity(data.len());

    for ele in data {
        let idx = cb.get(ele);
        match idx {
            Some(idx) => compressed.push(*idx),
            None => {
                let idx = cb.len();
                cb.insert(ele.clone(), idx as i64);
                compressed.push(idx as i64);
            }
        }
    }
    let cb = BTreeMap::<i64, String>::from_iter(cb.iter().map(|(k, v)| (*v, k.clone())));
    match simpler_compress(&compressed, 0) {
        Ok(compressed) => Ok((compressed, Some(cb))),
        Err(err) => Err(ProtoError::CompressError(err.to_string())),
    }
}

impl Decompressable for Seq {
    fn decompress(dtype: EleType, data: &[u8], cb: &CodeBook) -> Result<Self, ProtoError> {
        let seq = match dtype {
            EleType::Nil => Seq::Nil,
            EleType::I32 => Seq::SeqI32(
                simple_decompress::<i32>(data)
                    .map_err(|e| ProtoError::CompressError(e.to_string()))?,
            ),
            EleType::I64 => Seq::SeqI64(
                simple_decompress::<i64>(data)
                    .map_err(|e| ProtoError::CompressError(e.to_string()))?,
            ),
            EleType::F32 => Seq::SeqF32(
                simple_decompress::<f32>(data)
                    .map_err(|e| ProtoError::CompressError(e.to_string()))?,
            ),
            EleType::F64 => Seq::SeqF64(
                simple_decompress::<f64>(data)
                    .map_err(|e| ProtoError::CompressError(e.to_string()))?,
            ),
            EleType::Text | EleType::Url => {
                let data = simple_decompress::<i64>(data)
                    .map_err(|e| ProtoError::CompressError(e.to_string()))?;
                let cb = cb
                    .as_ref()
                    .ok_or(ProtoError::CompressError("missing codebook".to_string()))?;
                let data = data
                    .iter()
                    .map(|idx| cb.get(idx).cloned().unwrap_or_default())
                    .collect::<Vec<String>>();
                Seq::SeqText(data)
            }
            EleType::DataTime => Seq::SeqDateTime(
                simple_decompress::<u64>(data)
                    .map_err(|e| ProtoError::CompressError(e.to_string()))?,
            ),
        };
        Ok(seq)
    }
}

#[cfg(test)]
mod test {
    use crate::types::{compress::Decompressable, EleType, Seq};

    use super::text_compress;

    #[test]
    fn test_test_compress() {
        let seq = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "a".to_string(),
            "b".to_string(),
        ];

        let ret = text_compress(&seq);
        assert!(ret.is_ok());

        let (data, cb) = ret.unwrap();
        assert!(cb.is_some());

        let cb = cb.unwrap();
        assert_eq!(cb.get(&0), Some(&"a".to_string()));
        assert_eq!(cb.get(&1), Some(&"b".to_string()));
        assert_eq!(cb.get(&2), Some(&"c".to_string()));
        assert_eq!(cb.get(&3), Some(&"d".to_string()));
        assert_eq!(cb.get(&0), Some(&"a".to_string()));
        assert_eq!(cb.get(&1), Some(&"b".to_string()));

        let seq = Seq::decompress(EleType::Text, &data, &Some(cb)).unwrap();
        if let Seq::SeqText(seq) = seq {
            assert_eq!(seq, vec!["a", "b", "c", "d", "a", "b"]);
        } else {
            panic!("unexpected seq type");
        }
    }
}
