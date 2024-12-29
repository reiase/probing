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
    need_grow: bool,
}

impl Default for Series {
    fn default() -> Self {
        Series {
            chunk_size: 10000,
            offset: 0,
            dropped: 0,
            slices: Default::default(),
            need_grow: true,
        }
    }
}

impl Series {
    pub fn append<T>(&mut self, data: T) -> Result<()>
    where
        T: ArrayType,
    {
        if self.need_grow {
            let array = T::create_array(data, self.chunk_size);

            let offset = self.offset;

            self.slices.insert(
                offset,
                Slice {
                    offset: 0,
                    length: 0,
                    data: Page::Raw(array),
                },
            );
            self.need_grow = false;
        } else {
            let mut entry = self.slices.last_entry().unwrap();
            let slice = entry.get_mut();

            match slice.data {
                Page::Raw(ref mut array) => {
                    T::append_to_array(array, data)?;
                }
                Page::Compressed(_) => todo!(),
                Page::Ref => todo!(),
            }
            slice.length += 1;
            self.offset += 1;
            if slice.length == self.chunk_size {
                self.need_grow = true;
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.offset
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, idx: usize) -> Value {
        for (offset, slice) in self
            .slices
            .range((Included(&(idx - self.chunk_size)), Included(&idx)))
        {
            if idx < offset + slice.length {
                match &slice.data {
                    Page::Raw(array) => return array.get((idx - offset) as usize),
                    Page::Compressed(_) => todo!(),
                    Page::Ref => todo!(),
                }
            }
        }
        Value::Nil
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
    slice_iter: std::collections::btree_map::Iter<'a, usize, Slice>,
    current_slice: Option<(&'a usize, &'a Slice)>,
    elem_idx: usize,
}

impl<'a> SeriesIterator<'a> {
    pub fn new(series: &'a Series) -> Self {
        let mut slice_iter = series.slices.iter();
        let current_slice = slice_iter.next();
        SeriesIterator {
            slice_iter,
            current_slice,
            elem_idx: 0,
        }
    }
}

impl<'a> Iterator for SeriesIterator<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((_, slice)) = self.current_slice {
            if self.elem_idx < slice.length {
                let value = match &slice.data {
                    Page::Raw(array) => array.get(self.elem_idx),
                    Page::Compressed(_) => todo!(),
                    Page::Ref => todo!(),
                };
                self.elem_idx += 1;
                return Some(value);
            } else {
                self.current_slice = self.slice_iter.next();
                self.elem_idx = 0;
            }
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
            assert_eq!(series.get(i), super::Value::Int64(i as i64));
        }
    }

    #[test]
    fn test_series_iter() {
        let mut series = super::Series::default();
        for i in 0..512 {
            series.append(i as i64).unwrap();
        }

        for (i, value) in series.iter().enumerate() {
            assert_eq!(value, super::Value::Int64(i as i64));
        }
    }
}
