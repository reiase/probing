use anyhow::Result;

use crate::types::array::Array;

use super::Value;

pub enum Page {
    Raw(Array),
    Compressed(Vec<u8>),
    Ref,
}

pub struct Slice {
    pub offset: u64,
    pub length: u64,
    pub data: Page,
}

pub struct Series {
    pub chunk_size: usize,
    pub slices: Vec<Slice>,
}

impl Default for Series {
    fn default() -> Self {
        Series {
            chunk_size: 10000,
            slices: Vec::new(),
        }
    }
}

impl Series {
    pub fn append<T>(&mut self, data: T) -> Result<()>
    where
        T: Into<Value>,
    {
        let is_empty = self.slices.is_empty();

        if is_empty || self.slices.last().unwrap().length as usize >= self.chunk_size {
            let value: Value = data.into();
            fn new_array<X>(x: X, size: usize) -> Vec<X> {
                let mut array = Vec::with_capacity(size);
                array.push(x);
                array
            }
            let array = match value {
                Value::Nil => todo!(),
                Value::Int32(x) => Array::Int32Array(new_array(x, self.chunk_size)),
                Value::Int64(x) => Array::Int64Array(new_array(x, self.chunk_size)),
                Value::Float32(x) => Array::Float32Array(new_array(x, self.chunk_size)),
                Value::Float64(x) => Array::Float64Array(new_array(x, self.chunk_size)),
                Value::Text(x) => Array::TextArray(new_array(x, self.chunk_size)),
                Value::Url(x) => Array::TextArray(new_array(x, self.chunk_size)),
                Value::DataTime(x) => Array::DateTimeArray(new_array(x, self.chunk_size)),
            };
            self.slices.push(Slice {
                offset: if is_empty {
                    0
                } else {
                    self.slices.last().unwrap().offset + self.slices.last().unwrap().length
                },
                length: 1,
                data: Page::Raw(array),
            });
        } else {
            let slice = self.slices.last_mut().unwrap();

            match slice.data {
                Page::Raw(ref mut array) => {
                    let value: Value = data.into();
                    array.append(value)?;
                }
                Page::Compressed(_) => todo!(),
                Page::Ref => todo!(),
            }
            slice.length += 1;
        }
        Ok(())
    }

    pub fn len(&self) -> u64 {
        self.slices.iter().map(|s| s.length).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }

    pub fn get(&self, idx: u64) -> Value {
        let mut offset = 0;
        for slice in &self.slices {
            if idx < offset + slice.length {
                match &slice.data {
                    Page::Raw(array) => return array.get((idx - offset) as usize),
                    Page::Compressed(_) => todo!(),
                    Page::Ref => todo!(),
                }
            }
            offset += slice.length;
        }
        Value::Nil
    }
}

pub struct SeriesIterator<'a> {
    series: &'a Series,
    slice: u64,
    idx: u64,
}

impl<'a> Iterator for SeriesIterator<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice >= self.series.slices.len() as u64 {
            return None;
        }

        let slice = &self.series.slices[self.slice as usize];
        if slice.offset + self.idx >= slice.length {
            self.slice += 1;
            return self.next();
        }

        let value = match &slice.data {
            Page::Raw(array) => array.get((self.idx - slice.offset) as usize),
            Page::Compressed(_) => todo!(),
            Page::Ref => todo!(),
        };
        self.idx += 1;
        Some(value)
    }
}

impl<'a> IntoIterator for &'a Series {
    type Item = Value;
    type IntoIter = SeriesIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SeriesIterator {
            series: self,
            slice: 0,
            idx: 0,
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_new_series() {
        let series = super::Series::default();
        assert_eq!(series.chunk_size, 256);
        assert!(series.slices.is_empty());
    }

    #[test]
    fn test_series_append() {
        let mut series = super::Series::default();
        for i in 0..512 {
            series.append(i).unwrap();
        }
        assert_eq!(series.slices.len(), 2);

        assert_eq!(series.slices[0].length, 256);
        assert_eq!(series.slices[0].offset, 0);

        assert_eq!(series.slices[1].length, 256);
        assert_eq!(series.slices[1].offset, 256);
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

        for (i, value) in series.into_iter().enumerate() {
            assert_eq!(value, super::Value::Int64(i as i64));
        }
    }
}
