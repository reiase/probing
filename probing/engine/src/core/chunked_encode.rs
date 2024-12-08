use std::sync::Arc;

use anyhow::Result;
use arrow::array::{
    Array, Float32Array, Float64Array, Int32Array, Int64Array, RecordBatch, StringArray,
};
use serde::{ser::SerializeStruct, Serialize};

pub fn chunked_encode<E: Into<String>>(batch: &RecordBatch, encoder: E) -> Result<Vec<u8>> {
    let encoder = encoder.into();
    let chunk = &DataFrameChunk { chunk: batch };
    let encoded = match encoder.as_str() {
        "json" => serde_json::to_vec(chunk)?,
        "ron" => ron::to_string(chunk)?.as_bytes().to_vec(),
        _ => {
            return Err(anyhow::anyhow!("Unsupported encoder"));
        }
    };
    Ok(encoded)
}

pub struct DataFrameChunk<'a> {
    pub chunk: &'a RecordBatch,
}

pub struct ArrayChunk<'a> {
    pub chunk: &'a Arc<dyn Array>,
}

impl<'a> Serialize for DataFrameChunk<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut df = serializer.serialize_struct("DataFrame", 2)?;

        let names = self
            .chunk
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect::<Vec<_>>();
        df.serialize_field("names", &names)?;

        let cols = self
            .chunk
            .columns()
            .iter()
            .map(|c| ArrayChunk { chunk: c })
            .collect::<Vec<_>>();
        df.serialize_field("cols", &cols)?;

        df.end()
    }
}

impl<'a> Serialize for ArrayChunk<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(array) = self.chunk.as_any().downcast_ref::<Int32Array>() {
            serializer.serialize_newtype_variant("Array", 0, "Int32Array", &array.values().to_vec())
        } else if let Some(array) = self.chunk.as_any().downcast_ref::<Int64Array>() {
            serializer.serialize_newtype_variant("Array", 0, "Int64Array", &array.values().to_vec())
        } else if let Some(array) = self.chunk.as_any().downcast_ref::<Float32Array>() {
            serializer.serialize_newtype_variant(
                "Array",
                0,
                "Float32Array",
                &array.values().to_vec(),
            )
        } else if let Some(array) = self.chunk.as_any().downcast_ref::<Float64Array>() {
            serializer.serialize_newtype_variant(
                "Array",
                0,
                "Float64Array",
                &array.values().to_vec(),
            )
        } else if let Some(array) = self.chunk.as_any().downcast_ref::<StringArray>() {
            let values = (0..array.len())
                .map(|x| array.value(x).to_string())
                .collect::<Vec<_>>();
            serializer.serialize_newtype_variant("Array", 0, "TextArray", &values)
        } else {
            Err(serde::ser::Error::custom("Unsupported array type"))
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use arrow::array::ArrayRef;
    use arrow::array::Int32Array;
    use arrow::array::RecordBatch;
    use arrow::array::StringArray;

    use anyhow::Result;
    use probing_dpp::protocol::dataframe::Array;
    use probing_dpp::protocol::dataframe::DataFrame;

    use super::DataFrameChunk;

    #[test]
    fn test_ser_record_batch() -> Result<()> {
        let a: ArrayRef = Arc::new(Int32Array::from(vec![1, 2]));
        let b: ArrayRef = Arc::new(StringArray::from(vec!["a", "b"]));
        let chunk = RecordBatch::try_from_iter(vec![("a", a), ("b", b)])?;
        let chunk_str = ron::to_string(&DataFrameChunk { chunk: &chunk })?;

        let df = DataFrame {
            names: vec!["a".to_string(), "b".to_string()],
            cols: vec![
                Array::Int32Array(vec![1, 2]),
                Array::TextArray(vec!["a".to_string(), "b".to_string()]),
            ],
        };
        let df_str = ron::to_string(&df)?;

        assert_eq!(df_str, chunk_str);

        Ok(())
    }

    #[test]
    fn test_de_record_batch() -> Result<()> {
        let a: ArrayRef = Arc::new(Int32Array::from(vec![1, 2]));
        let b: ArrayRef = Arc::new(StringArray::from(vec!["a", "b"]));
        let chunk = RecordBatch::try_from_iter(vec![("a", a), ("b", b)])?;
        let chunk_str = ron::to_string(&DataFrameChunk { chunk: &chunk })?;

        let de_chunk: DataFrame = ron::from_str(&chunk_str)?;

        let df = DataFrame {
            names: vec!["a".to_string(), "b".to_string()],
            cols: vec![
                Array::Int32Array(vec![1, 2]),
                Array::TextArray(vec!["a".to_string(), "b".to_string()]),
            ],
        };
        let df_str = ron::to_string(&df)?;

        assert_eq!(df_str, ron::to_string(&de_chunk)?);

        Ok(())
    }
}
