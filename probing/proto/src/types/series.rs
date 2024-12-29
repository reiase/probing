use std::collections::BTreeMap;
use std::ops::Bound::Included;

use anyhow::Result;

use crate::types::array::Array;

use super::Value;

pub enum Page {
    Raw(Array),
    Compressed(Vec<u8>),
    Ref,
}

pub struct Slice {
    pub offset: usize,
    pub length: usize,
    pub data: Page,
}

pub struct Series {
    pub chunk_size: usize,
    pub offset: usize,
    pub dropped: usize,
    pub slices: BTreeMap<usize, Slice>,
    current_slice: Option<Slice>,
}

impl Default for Series {
    fn default() -> Self {
        Series {
            chunk_size: 10000,
            offset: 0,
            dropped: 0,
            slices: Default::default(),
            current_slice: None,
        }
    }
}

impl Series {
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
                if slice.length == self.chunk_size {
                    let slice = std::mem::replace(&mut self.current_slice, None);
                    if let Some(slice) = slice {
                        self.slices.insert(slice.offset, slice);
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Current page is not Raw"));
            }
        } else {
            let array = T::create_array(data, self.chunk_size);
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
            if idx >= slice.offset && idx < slice.offset + slice.length {
                match &slice.data {
                    Page::Raw(array) => return Some(array.get(idx - slice.offset)),
                    Page::Compressed(_) => return None, // TODO: implement decompression
                    Page::Ref => return None,           // TODO: implement reference resolution
                }
            }
        }

        // Search in BTreeMap
        let start = idx.saturating_sub(self.chunk_size);
        for (offset, slice) in self.slices.range((Included(&start), Included(&idx))) {
            if idx >= *offset && idx < offset + slice.length {
                match &slice.data {
                    Page::Raw(array) => return Some(array.get(idx - offset)),
                    Page::Compressed(_) => return None, // TODO: implement decompression
                    Page::Ref => return None,           // TODO: implement reference resolution
                }
            }
        }

        None
    }

    pub fn iter(&self) -> SeriesIterator {
        SeriesIterator::new(self)
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
    series: &'a Series,
    current_btree_iter: std::collections::btree_map::Range<'a, usize, Slice>,
    current_btree_slice: Option<(&'a usize, &'a Slice)>,
    current_slice: Option<&'a Slice>,
    elem_idx: usize,
}

impl<'a> SeriesIterator<'a> {
    pub fn new(series: &'a Series) -> Self {
        let start = series.dropped;
        let end = series.offset;
        SeriesIterator {
            series,
            current_btree_iter: series.slices.range((Included(&start), Included(&end))),
            current_btree_slice: None,
            current_slice: series.current_slice.as_ref(),
            elem_idx: 0,
        }
    }
}

impl<'a> Iterator for SeriesIterator<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        // Process BTreeMap slices first
        while let Some((_, slice)) = self.current_btree_slice {
            if self.elem_idx < slice.length {
                if let Page::Raw(ref array) = slice.data {
                    let value = array.get(self.elem_idx);
                    self.elem_idx += 1;
                    return Some(value);
                }
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
        let series = super::Series::default();
        assert_eq!(series.chunk_size, 10000);
        assert!(series.slices.is_empty());
    }

    #[test]
    fn test_series_append() {
        let mut series = super::Series::default();
        series.chunk_size = 256;
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
        let mut series = super::Series::default();
        for i in 0..512 {
            series.append(i as i64).unwrap();
        }

        for i in 0..512 {
            assert_eq!(series.get(i).unwrap(), super::Value::Int64(i as i64));
        }
    }

    #[test]
    fn test_series_iter() {
        let mut series = super::Series::default();
        for i in 0..512 {
            series.append(i as i64).unwrap();
        }

        assert_eq!(512, series.iter().collect::<Vec<_>>().len());

        for (i, value) in series.iter().enumerate() {
            assert_eq!(value, super::Value::Int64(i as i64));
        }
    }
}
