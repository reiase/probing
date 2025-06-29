use std::collections::BTreeMap;
use std::ops::Bound::Included;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::CodeBook;
use super::Compressable;
use super::Decompressable;
use super::Ele;
use super::EleType;
use super::ProtoError;
use super::Seq;

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Page {
    Raw(Seq),
    Compressed {
        dtype: EleType,
        buffer: Vec<u8>,
        codebook: CodeBook,
    },
    Ref,
}

impl Page {
    pub fn nbytes(&self) -> usize {
        match self {
            Page::Raw(array) => array.nbytes(),
            Page::Compressed { buffer, .. } => buffer.len(),
            Page::Ref => 0,
        }
    }

    pub fn decompress_buffer(&self, dtype: EleType, buffer: &[u8], cb: &CodeBook) -> Option<Page> {
        let seq = Seq::decompress(dtype, buffer, cb);
        seq.map(Page::Raw).ok()
    }

    pub fn get_value(&self, page_offset: usize) -> Option<Ele> {
        match self {
            Page::Raw(array) => Some(array.get(page_offset)),
            Page::Compressed {
                dtype,
                buffer,
                codebook,
            } => {
                if let Some(page) = self.decompress_buffer(dtype.clone(), buffer, codebook) {
                    return page.get_value(page_offset);
                }
                None
            }
            Page::Ref => Some(Ele::Nil),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct Slice {
    pub offset: usize,
    pub length: usize,
    pub data: Page,
}

impl Slice {
    pub fn nbytes(&self) -> usize {
        self.data.nbytes()
    }

    pub fn get_value(&self, slice_offset: usize) -> Option<Ele> {
        self.data.get_value(slice_offset)
    }

    pub fn get_with_index(&self, idx: usize) -> Option<Ele> {
        self.get_value(idx - self.offset)
    }

    pub fn compress(&mut self) {
        if let Page::Raw(array) = &self.data {
            if let Ok((dtype, buffer, codebook)) = array.compress() {
                self.data = Page::Compressed {
                    dtype,
                    buffer,
                    codebook,
                };
            }
        }
    }

    pub fn decompress(&mut self) {
        if let Page::Compressed {
            dtype,
            buffer,
            codebook,
        } = &self.data
        {
            if let Some(decompressed) = self.data.decompress_buffer(dtype.clone(), buffer, codebook)
            {
                self.data = decompressed;
            }
        }
    }
}

/// Configuration for Series data storage and compression
///
/// Controls how Series data is chunked, compressed, and managed in memory.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct SeriesConfig {
    pub dtype: EleType,
    pub chunk_size: usize,
    pub compression_level: usize,
    pub compression_threshold: usize,
    pub discard_threshold: usize,
}

impl Default for SeriesConfig {
    fn default() -> Self {
        SeriesConfig {
            dtype: EleType::Nil,
            chunk_size: 10000,
            compression_level: 0,
            compression_threshold: 2_000_000,
            discard_threshold: 20_000_000,
        }
    }
}

impl SeriesConfig {
    pub fn with_dtype(mut self, dtype: EleType) -> Self {
        self.dtype = dtype;
        self
    }
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }
    pub fn with_compression_level(mut self, compression_level: usize) -> Self {
        self.compression_level = compression_level;
        self
    }
    pub fn with_compression_threshold(mut self, compression_threshold: usize) -> Self {
        self.compression_threshold = compression_threshold;
        self
    }
    pub fn with_discard_threshold(mut self, discard_threshold: usize) -> Self {
        self.discard_threshold = discard_threshold;
        self
    }
    pub fn build(self) -> Series {
        Series {
            config: self,
            offset: 0,
            dropped: 0,
            slices: Default::default(),
            current_slice: None,
            commit_nbytes: 0,
        }
    }
}

/// Time series data structure with compression and memory management
///
/// Series stores sequences of homogeneous data with automatic chunking,
/// optional compression, and automatic pruning of old data when memory
/// thresholds are exceeded.
#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Clone)]
pub struct Series {
    config: SeriesConfig,
    pub offset: usize,
    pub dropped: usize,
    pub slices: BTreeMap<usize, Slice>,
    current_slice: Option<Slice>,

    commit_nbytes: usize,
}

impl Series {
    pub fn builder() -> SeriesConfig {
        SeriesConfig::default()
    }

    pub fn append<T>(&mut self, data: T) -> Result<(), ProtoError>
    where
        T: ArrayType,
    {
        if self.offset == usize::MAX {
            return Err(ProtoError::CapacityExceeded);
        }

        if let Some(slice) = self.current_slice.as_mut() {
            if let Page::Raw(ref mut array) = slice.data {
                T::append_to_array(array, data)?;
                slice.length += 1;
                if slice.length == self.config.chunk_size {
                    self.commit_current_slice();
                }
            } else {
                return Err(ProtoError::RawPageTypeExpected);
            }
        } else {
            self.config.dtype = T::dtype();

            let array = T::create_array(data, self.config.chunk_size);
            let page = Page::Raw(array);
            let offset = self.offset;

            self.current_slice = Some(Slice {
                offset,
                length: 1,
                data: page,
            });
        }

        self.offset = self.offset.saturating_add(1);
        Ok(())
    }

    pub fn append_value(&mut self, data: Ele) -> Result<(), ProtoError> {
        match data {
            Ele::I32(data) => self.append(data),
            Ele::I64(data) => self.append(data),
            Ele::F32(data) => self.append(data),
            Ele::F64(data) => self.append(data),
            Ele::Text(data) => self.append(data),
            _ => Err(ProtoError::InvalidValueDateType),
        }
    }

    pub fn dtype(&self) -> EleType {
        self.config.dtype.clone()
    }

    pub fn len(&self) -> usize {
        self.offset
    }

    pub fn nbytes(&self) -> usize {
        let mut total = self.commit_nbytes;

        // Add bytes from current slice if exists
        if let Some(slice) = &self.current_slice {
            total += slice.nbytes();
        }

        total
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, idx: usize) -> Option<Ele> {
        // Check if index is out of range
        if idx >= self.offset || idx < self.dropped {
            return None;
        }

        // Check current slice first
        if let Some(slice) = self.current_slice.as_ref() {
            if idx >= slice.offset && idx < slice.offset + slice.length {
                return slice.get_with_index(idx);
            }
        }

        // Search in BTreeMap
        let start = idx.saturating_sub(self.config.chunk_size);

        for (offset, slice) in self.slices.range((Included(&start), Included(&idx))) {
            if idx >= *offset && idx < offset + slice.length {
                return slice.get_value(idx - offset);
            }
        }

        None
    }

    pub fn iter(&self) -> SeriesIterator<'_> {
        SeriesIterator::new(self)
    }
}

impl Series {
    fn commit_current_slice(&mut self) {
        let nbytes = self.nbytes();
        let slice = self.current_slice.take();
        if nbytes > self.config.compression_threshold {
            if let Some(mut slice) = slice {
                slice.compress();
                self.commit_nbytes += slice.nbytes();
                self.slices.insert(slice.offset, slice);
            }
        } else if let Some(slice) = slice {
            self.commit_nbytes += slice.nbytes();
            self.slices.insert(slice.offset, slice);
        }

        while self.nbytes() > self.config.discard_threshold {
            if let Some((_offset, slice)) = self.slices.pop_first() {
                self.dropped += slice.offset + slice.length;
                self.commit_nbytes -= slice.nbytes();
            }
        }
    }
}

pub trait ArrayType {
    fn dtype() -> EleType;
    fn create_array(data: Self, size: usize) -> Seq;
    fn append_to_array(array: &mut Seq, data: Self) -> Result<(), ProtoError>;
}

macro_rules! impl_array_type {
    ($type:ty, $ele_type:ident, $seq_variant:ident) => {
        impl ArrayType for $type {
            fn dtype() -> EleType {
                EleType::$ele_type
            }

            fn create_array(data: Self, size: usize) -> Seq {
                let mut array = Vec::with_capacity(size);
                array.push(data);
                Seq::$seq_variant(array)
            }

            fn append_to_array(array: &mut Seq, data: Self) -> Result<(), ProtoError> {
                if let Seq::$seq_variant(arr) = array {
                    arr.push(data);
                    Ok(())
                } else {
                    Err(ProtoError::TypeMismatch {
                        expected: EleType::$ele_type,
                        got: EleType::Nil,
                    })
                }
            }
        }
    };
}

// 使用宏实现所有基本类型
impl_array_type!(i32, I32, SeqI32);
impl_array_type!(i64, I64, SeqI64);
impl_array_type!(f32, F32, SeqF32);
impl_array_type!(f64, F64, SeqF64);
impl_array_type!(String, Text, SeqText);

pub struct SeriesIterator<'a> {
    current_btree_iter: std::collections::btree_map::Iter<'a, usize, Slice>,
    current_btree_slice: Option<(&'a usize, &'a Slice)>,
    current_slice: Option<&'a Slice>,
    elem_idx: usize,

    cache: Seq,
}

impl<'a> SeriesIterator<'a> {
    pub fn new(series: &'a Series) -> Self {
        let mut current_btree_iter = series.slices.iter();
        let current_btree_slice = current_btree_iter.next();
        SeriesIterator {
            current_btree_iter,
            current_btree_slice,
            current_slice: series.current_slice.as_ref(),
            elem_idx: 0,
            cache: Seq::Nil,
        }
    }

    fn get_value_from_slice(&mut self, slice: &Slice) -> Option<Ele> {
        if self.elem_idx >= slice.length {
            return None;
        }

        // Handle decompression for compressed slices on first access
        if self.elem_idx == 0 {
            if let Page::Compressed {
                dtype,
                buffer,
                codebook,
            } = &slice.data
            {
                if let Some(page) = slice
                    .data
                    .decompress_buffer(dtype.clone(), buffer, codebook)
                {
                    self.cache = if let Page::Raw(array) = page {
                        array
                    } else {
                        Seq::Nil
                    }
                }
            }
        }

        let array = match &slice.data {
            Page::Raw(array) => array,
            Page::Compressed { .. } => &self.cache,
            Page::Ref => return Some(Ele::Nil),
        };

        let value = array.get(self.elem_idx);
        self.elem_idx += 1;
        Some(value)
    }
}

impl Iterator for SeriesIterator<'_> {
    type Item = Ele;

    fn next(&mut self) -> Option<Self::Item> {
        // Process BTreeMap slices first
        while let Some((_, slice)) = self.current_btree_slice {
            if let Some(value) = self.get_value_from_slice(slice) {
                return Some(value);
            }
            // Move to next slice
            self.current_btree_slice = self.current_btree_iter.next();
            self.elem_idx = 0;
        }

        // Then try current_slice
        if let Some(slice) = self.current_slice {
            if let Some(value) = self.get_value_from_slice(slice) {
                return Some(value);
            }
            // Done with current_slice
            self.current_slice = None;
            self.elem_idx = 0;
        }

        None
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_new_series() {
        let series = super::Series::builder().build();
        assert_eq!(series.config.chunk_size, 10000);
        assert!(series.slices.is_empty());
    }

    #[test]
    fn test_series_append() {
        let mut series = super::Series::builder().with_chunk_size(256).build();
        for i in 0..512 {
            series.append(i).unwrap();
        }
        assert_eq!(series.slices.len(), 2);

        assert_eq!(series.slices.get(&0).unwrap().length, 256);
        assert_eq!(series.slices.get(&0).unwrap().offset, 0);

        assert_eq!(series.slices.get(&256).unwrap().length, 256);
        assert_eq!(series.slices.get(&256).unwrap().offset, 256);
    }

    #[test]
    fn test_series_get() {
        let mut series = super::Series::builder().with_chunk_size(256).build();

        for i in 0..512 {
            series.append(i as i64).unwrap();
        }

        for i in 1..512 {
            assert_eq!(series.get(i).unwrap(), super::Ele::I64(i as i64));
        }
    }

    #[test]
    fn test_series_get_from_compressed() {
        let mut series = super::Series::builder()
            .with_compression_threshold(8)
            .with_chunk_size(256)
            .build();

        for i in 0..512 {
            series.append(i as i64).unwrap();
        }

        for i in 1..512 {
            assert_eq!(series.get(i).unwrap(), super::Ele::I64(i as i64));
        }
    }

    #[test]
    fn test_series_iter() {
        let mut series = super::Series::builder().with_chunk_size(256).build();
        let mut expected_sum = 0;
        for i in 0..512 {
            series.append(i).unwrap();
            expected_sum += i;
        }

        for (i, value) in series.iter().enumerate() {
            assert_eq!(value, super::Ele::I64(i as i64));
        }

        assert_eq!(512, series.iter().collect::<Vec<_>>().len());
        assert_eq!(
            expected_sum,
            series
                .iter()
                .map(|v| TryInto::<i64>::try_into(v).unwrap())
                .sum::<i64>()
        );
    }

    #[test]
    fn test_series_nbytes() {
        /// Test compression effectiveness for different data types
        fn test_nbytes_for_type<T>(
            values: impl Iterator<Item = T> + Clone,
            type_name: &str,
            type_size: usize,
        ) where
            T: super::ArrayType,
        {
            let mut series = super::Series::builder()
                .with_compression_threshold(8)
                .with_chunk_size(256)
                .build();

            for value in values {
                series.append(value).unwrap();
            }

            println!("512 {} nbytes: {}", type_name, series.nbytes());
            assert!(
                series.nbytes() * 5 < 512 * type_size,
                "Compression not effective enough for {type_name} type"
            );
        }

        // Test different data types with their respective iterators and sizes
        test_nbytes_for_type(
            (0..512).map(|i| i as i64),
            "i64",
            std::mem::size_of::<i64>(),
        );
        test_nbytes_for_type(0..512, "i32", std::mem::size_of::<i32>());
        test_nbytes_for_type(
            (0..512).map(|i| i as f32),
            "f32",
            std::mem::size_of::<f32>(),
        );
        test_nbytes_for_type(
            (0..512).map(|i| i as f64),
            "f64",
            std::mem::size_of::<f64>(),
        );
    }

    #[test]
    fn test_drop_history() {
        let mut series = super::Series::builder()
            .with_chunk_size(256)
            .with_compression_threshold(128)
            .with_discard_threshold(200)
            .build();

        for i in 0..1024 {
            series.append(i as i64).unwrap();
        }

        // Initially should have 4 chunks of 256 elements each
        assert_eq!(series.slices.len(), 4);

        // Add more data to trigger dropping
        for i in 1024..2048 {
            series.append(i as i64).unwrap();
        }

        // Some older chunks should be dropped
        assert!(series.slices.len() < 8);
        assert!(series.dropped > 0);

        // Verify that dropped elements cannot be accessed
        assert!(series.get(0).is_none());

        // But newer elements can still be accessed
        assert!(series.get(series.dropped + 1).is_some());
    }

    #[test]
    fn test_series_serialization() {
        // Create a series and add some data
        let mut original_series = super::Series::builder()
            .with_chunk_size(256)
            .with_compression_threshold(5)
            .build();

        for i in 0..500 {
            original_series.append(i as i64).unwrap();
        }

        // Serialize to JSON
        let serialized = serde_json::to_string(&original_series).unwrap();

        // Deserialize back
        let deserialized_series: super::Series = serde_json::from_str(&serialized).unwrap();

        // Verify the series are equal
        assert_eq!(original_series, deserialized_series);

        // Verify data can still be accessed
        for i in 0..500 {
            assert_eq!(
                deserialized_series.get(i).unwrap(),
                super::Ele::I64(i as i64)
            );
        }

        // Verify compressed slices are preserved
        assert_eq!(
            original_series.slices.len(),
            deserialized_series.slices.len()
        );
        assert_eq!(original_series.nbytes(), deserialized_series.nbytes());
    }
}
