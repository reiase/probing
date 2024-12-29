use std::collections::BTreeMap;
use std::ops::Bound::Included;

use anyhow::Result;
use pco::standalone::simple_decompress;
use pco::standalone::simpler_compress;

use crate::types::array::Array;

use super::value::DataType;
use super::Value;

pub enum Page {
    Raw(Array),
    Compressed { dtype: DataType, buffer: Vec<u8> },
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

    pub fn compress_buffer(&self, array: &Array) -> Option<(DataType, Vec<u8>)> {
        match array {
            Array::Int32Array(data) => {
                let compressed = simpler_compress(data.as_slice(), 0);
                match compressed {
                    Ok(mut compressed) => {
                        compressed.shrink_to_fit();
                        return Some((DataType::Int32, compressed));
                    }
                    Err(_) => return None,
                }
            }
            Array::Int64Array(data) => {
                let compressed = simpler_compress(data.as_slice(), 0);
                match compressed {
                    Ok(mut compressed) => {
                        compressed.shrink_to_fit();
                        return Some((DataType::Int64, compressed));
                    }
                    Err(_) => return None,
                }
            }
            Array::Float32Array(data) => {
                let compressed = simpler_compress(data.as_slice(), 0);
                match compressed {
                    Ok(mut compressed) => {
                        compressed.shrink_to_fit();
                        return Some((DataType::Float32, compressed));
                    }
                    Err(_) => return None,
                }
            }
            Array::Float64Array(data) => {
                let compressed = simpler_compress(data.as_slice(), 0);
                match compressed {
                    Ok(mut compressed) => {
                        compressed.shrink_to_fit();
                        return Some((DataType::Float64, compressed));
                    }
                    Err(_) => return None,
                }
            }
            _ => return None,
        }
    }

    pub fn decompress_buffer(&self, dtype: DataType, buffer: &Vec<u8>) -> Option<Page> {
        match dtype {
            DataType::Int32 => {
                if let Ok(data) = simple_decompress::<i32>(buffer.as_slice()) {
                    return Some(Page::Raw(Array::Int32Array(data)));
                }
                return None;
            }
            DataType::Int64 => {
                if let Ok(data) = simple_decompress::<i64>(buffer.as_slice()) {
                    return Some(Page::Raw(Array::Int64Array(data)));
                }
                return None;
            }
            DataType::Float32 => {
                if let Ok(data) = simple_decompress::<f32>(buffer.as_slice()) {
                    return Some(Page::Raw(Array::Float32Array(data)));
                }
                return None;
            }
            DataType::Float64 => {
                if let Ok(data) = simple_decompress::<f64>(buffer.as_slice()) {
                    return Some(Page::Raw(Array::Float64Array(data)));
                }
                return None;
            }
            _ => return None,
        }
    }

    pub fn get_value(&self, page_offset: usize) -> Option<Value> {
        match self {
            Page::Raw(array) => Some(array.get(page_offset)),
            Page::Compressed { dtype, buffer } => {
                if let Some(page) = self.decompress_buffer(dtype.clone(), buffer) {
                    return page.get_value(page_offset);
                }
                None
            }
            Page::Ref => Some(Value::Nil),
        }
    }
}

pub struct Slice {
    pub offset: usize,
    pub length: usize,
    pub data: Page,
}

impl Slice {
    pub fn nbytes(&self) -> usize {
        self.data.nbytes()
    }

    pub fn get_value(&self, slice_offset: usize) -> Option<Value> {
        self.data.get_value(slice_offset)
    }

    pub fn get_with_index(&self, idx: usize) -> Option<Value> {
        self.data.get_value(idx - self.offset)
    }

    pub fn compress(&mut self) {
        if let Page::Raw(array) = &self.data {
            if let Some((dtype, buffer)) = self.data.compress_buffer(array) {
                self.data = Page::Compressed { dtype, buffer };
            }
        }
    }

    pub fn decompress(&mut self) {
        if let Page::Compressed { dtype, buffer } = &self.data {
            if let Some(decompressed) = self.data.decompress_buffer(dtype.clone(), buffer) {
                self.data = decompressed;
            }
        }
    }
}

pub struct SeriesConfig {
    pub dtype: DataType,
    pub chunk_size: usize,
    pub compression_level: usize,
    pub compression_threshold: usize,
}

impl Default for SeriesConfig {
    fn default() -> Self {
        SeriesConfig {
            dtype: DataType::Int64,
            chunk_size: 10000,
            compression_level: 0,
            compression_threshold: 2_000_000,
        }
    }
}

impl SeriesConfig {
    pub fn with_dtype(mut self, dtype: DataType) -> Self {
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
    pub fn build(self) -> Series {
        Series {
            config: self,
            offset: 0,
            dropped: 0,
            slices: Default::default(),
            current_slice: None,
        }
    }
}

pub struct Series {
    config: SeriesConfig,
    pub offset: usize,
    pub dropped: usize,
    pub slices: BTreeMap<usize, Slice>,
    current_slice: Option<Slice>,
}

impl Default for Series {
    fn default() -> Self {
        Series {
            config: Default::default(),
            offset: 0,
            dropped: 0,
            slices: Default::default(),
            current_slice: None,
        }
    }
}

impl Series {
    pub fn builder() -> SeriesConfig {
        SeriesConfig::default()
    }

    pub fn append<T>(&mut self, data: T) -> Result<()>
    where
        T: ArrayType,
    {
        if self.offset == usize::MAX {
            return Err(anyhow::anyhow!("Series capacity exceeded"));
        }

        if let Some(slice) = self.current_slice.as_mut() {
            if let Page::Raw(ref mut array) = slice.data {
                T::append_to_array(array, data)?;
                slice.length += 1;
                if slice.length == self.config.chunk_size {
                    self.commit_current_slice();
                }
            } else {
                return Err(anyhow::anyhow!("Current page is not Raw"));
            }
        } else {
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

    pub fn len(&self) -> usize {
        self.offset
    }

    pub fn nbytes(&self) -> usize {
        let mut total = 0;

        // Add bytes from historical slices
        for slice in self.slices.values() {
            total += slice.data.nbytes();
        }

        // Add bytes from current slice if exists
        if let Some(slice) = &self.current_slice {
            total += slice.data.nbytes();
        }

        total
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, idx: usize) -> Option<Value> {
        // Check if index is out of range
        if idx >= self.offset || idx < self.dropped {
            return None;
        }

        // Check current slice first
        if let Some(slice) = self.current_slice.as_ref() {
            return slice.get_with_index(idx);
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

    pub fn iter(&self) -> SeriesIterator {
        SeriesIterator::new(self)
    }
}

impl Series {
    fn commit_current_slice(&mut self) {
        let slice = std::mem::replace(&mut self.current_slice, None);
        if let Some(mut slice) = slice {
            slice.compress();
            self.slices.insert(slice.offset, slice);
        }
    }
}

pub trait ArrayType {
    fn create_array(data: Self, size: usize) -> Array;
    fn append_to_array(array: &mut Array, data: Self) -> Result<()>;
}

impl ArrayType for i32 {
    fn create_array(data: Self, size: usize) -> Array {
        let mut array = Vec::with_capacity(size);
        array.push(data);
        Array::Int32Array(array)
    }

    fn append_to_array(array: &mut Array, data: Self) -> Result<()> {
        if let Array::Int32Array(arr) = array {
            arr.push(data);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Type mismatch"))
        }
    }
}

impl ArrayType for i64 {
    fn create_array(data: Self, size: usize) -> Array {
        let mut array = Vec::with_capacity(size);
        array.push(data);
        Array::Int64Array(array)
    }

    fn append_to_array(array: &mut Array, data: Self) -> Result<()> {
        if let Array::Int64Array(arr) = array {
            arr.push(data);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Type mismatch"))
        }
    }
}

pub struct SeriesIterator<'a> {
    current_btree_iter: std::collections::btree_map::Iter<'a, usize, Slice>,
    current_btree_slice: Option<(&'a usize, &'a Slice)>,
    current_slice: Option<&'a Slice>,
    elem_idx: usize,

    cache: Array,
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
            cache: Array::Nil,
        }
    }
}

impl<'a> Iterator for SeriesIterator<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        // Process BTreeMap slices first
        while let Some((_, slice)) = self.current_btree_slice {
            if self.elem_idx < slice.length {
                if self.elem_idx == 0 {
                    if let Page::Compressed { dtype, buffer } = &slice.data {
                        if let Some(page) = slice.data.decompress_buffer(dtype.clone(), buffer) {
                            self.cache = if let Page::Raw(array) = page {
                                array
                            } else {
                                Array::Nil
                            }
                        }
                    }
                }
                let array = if let Page::Raw(ref array) = slice.data {
                    array
                } else {
                    &self.cache
                };
                let value = array.get(self.elem_idx);
                self.elem_idx += 1;
                return Some(value);
            }
            self.current_btree_slice = self.current_btree_iter.next();
            self.elem_idx = 0;
        }

        // Then try current_slice
        if let Some(slice) = self.current_slice {
            if self.elem_idx < slice.length {
                if let Page::Raw(ref array) = slice.data {
                    let value = array.get(self.elem_idx);
                    self.elem_idx += 1;
                    return Some(value);
                }
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
            assert_eq!(series.get(i).unwrap(), super::Value::Int64(i as i64));
        }
    }

    #[test]
    fn test_series_iter() {
        let mut series = super::Series::builder().with_chunk_size(256).build();
        let mut expected_sum = 0;
        for i in 0..512 {
            series.append(i as i64).unwrap();
            expected_sum += i;
        }

        for (i, value) in series.iter().enumerate() {
            assert_eq!(value, super::Value::Int64(i as i64));
        }

        assert_eq!(512, series.iter().collect::<Vec<_>>().len());
        assert_eq!(expected_sum, series.iter().map(|v| TryInto::<i64>::try_into(v).unwrap()).sum::<i64>());

        // for (i, value) in series.iter().enumerate() {
        //     assert_eq!(value, super::Value::Int64(i as i64));
        // }
    }

    #[test]
    fn test_series_nbytes() {
        let mut series = super::Series::builder().with_chunk_size(256).build();

        for i in 0..512 {
            series.append(i as i64).unwrap();
        }

        assert!(series.nbytes() * 5 < 512 * std::mem::size_of::<i64>());
    }
}
