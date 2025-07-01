use serde::{Deserialize, Serialize};

use super::Ele;
use super::Seq;

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Clone)]
pub struct DataFrame {
    pub names: Vec<String>,
    pub cols: Vec<Seq>,
    pub size: u64,
}

impl DataFrame {
    pub fn new(names: Vec<String>, columns: Vec<Seq>) -> Self {
        DataFrame {
            names,
            cols: columns,
            size: 0,
        }
    }

    pub fn len(&self) -> usize {
        if self.cols.is_empty() {
            return 0;
        }
        self.cols[0].len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&'_ self) -> DataFrameIterator<'_> {
        DataFrameIterator {
            df: self,
            current: 0,
        }
    }
}

pub struct DataFrameIterator<'a> {
    df: &'a DataFrame,
    current: usize,
}

impl Iterator for DataFrameIterator<'_> {
    type Item = Vec<Ele>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.df.len() {
            None
        } else {
            let mut row = vec![];
            for i in 0..self.df.cols.len() {
                row.push(self.df.cols[i].get(self.current));
            }
            self.current += 1;
            Some(row)
        }
    }
}
