use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::error::ProtoError;
use super::series::{DiscardStrategy, SeriesIterator};
use super::{basic::EleType, series::SeriesConfig, Ele, Series};

// use pyo3::types::PyDict;

#[derive(Debug, Error)]
pub enum TimeSeriesError {
    #[error("column count mismatch")]
    ColumnCountMismatch { expected: usize, got: usize },
    #[error("column type mismatch")]
    ColumnTypeMismatch { expected: EleType, got: EleType },
    #[error("invalid timestamp type")]
    InvalidTimestampType,
    #[error("unkown error")]
    UnknownError(String),
}

impl From<ProtoError> for TimeSeriesError {
    fn from(err: ProtoError) -> Self {
        match err {
            ProtoError::TypeMismatch { expected, got } => {
                TimeSeriesError::ColumnTypeMismatch { expected, got }
            }
            _ => TimeSeriesError::UnknownError(err.to_string()),
        }
    }
}

/// A time series is multiple series shares the same timestamp.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct TimeSeries {
    pub names: Vec<String>,
    pub timestamp: Series,
    pub cols: Vec<Series>,
}

impl TimeSeries {
    pub fn builder(limit: usize) -> TimeSeriesConfig {
        let ts_config = TimeSeriesConfig::default()
        .with_discard_threshold(limit)
        .with_chunk_size(limit)
        .with_discard_strategy(DiscardStrategy::BaseElementCount);
        ts_config
    }

    pub fn len(&self) -> usize {
        self.timestamp.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn append(&mut self, timestamp: Ele, values: Vec<Ele>) -> Result<(), TimeSeriesError> {
        if self.cols.len() != values.len() {
            return Err(TimeSeriesError::ColumnCountMismatch {
                expected: self.cols.len(),
                got: values.len(),
            });
        }
        self.timestamp.append_value(timestamp)?;
        for (i, item) in values.iter().enumerate().take(self.cols.len()) {
            println!("{} ts ready append value", i);
            self.cols[i].append_value(item.clone())?;
        }
        Ok(())
    }

    pub fn iter(&self) -> TimeSeriesIter {
        TimeSeriesIter {
            timestamp: self.timestamp.iter(),
            cols: self.cols.iter().map(|s| s.iter()).collect(),
        }
    }

    pub fn take(&self, limit: Option<usize>) -> Vec<(Ele, Vec<Ele>)> {
        let iter = self.iter();
        if let Some(limit) = limit {
            iter.take(limit).collect::<Vec<_>>()
        } else {
            iter.collect::<Vec<_>>()
        }
    }
}

#[derive(Default)]
pub struct TimeSeriesConfig {
    series_config: SeriesConfig,
    names: Vec<String>,
}

impl TimeSeriesConfig {
    pub fn with_dtype(mut self, dtype: EleType) -> Self {
        self.series_config = self.series_config.with_dtype(dtype);
        self
    }
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.series_config = self.series_config.with_chunk_size(chunk_size);
        self
    }
    pub fn with_compression_level(mut self, compression_level: usize) -> Self {
        self.series_config = self.series_config.with_compression_level(compression_level);
        self
    }
    pub fn with_compression_threshold(mut self, compression_threshold: usize) -> Self {
        self.series_config = self
            .series_config
            .with_compression_threshold(compression_threshold);
        self
    }
    pub fn with_discard_threshold(mut self, discard_threshold: usize) -> Self {
        self.series_config = self.series_config.with_discard_threshold(discard_threshold);
        self
    }
    pub fn with_discard_strategy(mut self, discard_strategy: DiscardStrategy) -> Self {
        self.series_config = self.series_config.with_discard_strategy(discard_strategy);
        self
    }
    pub fn with_columns(mut self, names: Vec<String>) -> Self {
        self.names = names;
        self
    }
    pub fn build(self) -> TimeSeries {
        let cols = self
            .names
            .iter()
            .map(|_| self.series_config.clone().build())
            .collect::<Vec<_>>();
        TimeSeries {
            names: self.names,
            timestamp: self.series_config.clone().build(),
            cols,
        }
    }
}

pub struct TimeSeriesIter<'a> {
    timestamp: SeriesIterator<'a>,
    cols: Vec<SeriesIterator<'a>>,
}

impl Iterator for TimeSeriesIter<'_> {
    type Item = (Ele, Vec<Ele>);

    fn next(&mut self) -> Option<Self::Item> {
        let timestamp = self.timestamp.next()?;
        let cols = self
            .cols
            .iter_mut()
            .map(|s| s.next())
            .collect::<Option<Vec<_>>>()?;
        Some((timestamp, cols))
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_timeseries_create() {
        let _ = super::TimeSeries::builder()
            .with_dtype(super::EleType::I64)
            .with_chunk_size(10)
            .with_compression_level(1)
            .with_compression_threshold(10)
            .with_discard_threshold(10)
            .with_columns(vec!["a".to_string(), "b".to_string()])
            .build();
    }

    #[test]
    fn test_timeseries_append() {
        let mut ts = super::TimeSeries::builder()
            .with_dtype(super::EleType::I64)
            .with_chunk_size(10)
            .with_compression_level(1)
            .with_compression_threshold(10)
            .with_discard_threshold(10)
            .with_columns(vec!["a".to_string(), "b".to_string()])
            .build();
        let _ = ts.append(
            super::Ele::I64(1),
            vec![super::Ele::I64(1), super::Ele::I64(2)],
        );
    }
    #[test]
    fn test_timeseries_iter() {
        let mut ts = super::TimeSeries::builder()
            .with_dtype(super::EleType::I64)
            .with_chunk_size(10)
            .with_compression_level(1)
            .with_compression_threshold(10)
            .with_discard_threshold(10)
            .with_columns(vec!["a".to_string(), "b".to_string()])
            .build();

        // Append some test data
        ts.append(
            super::Ele::I64(1),
            vec![super::Ele::I64(10), super::Ele::I64(20)],
        )
        .unwrap();
        ts.append(
            super::Ele::I64(2),
            vec![super::Ele::I64(30), super::Ele::I64(40)],
        )
        .unwrap();

        // Test iteration
        let mut iter = ts.iter();

        let (t1, v1) = iter.next().unwrap();
        assert_eq!(t1, super::Ele::I64(1));
        assert_eq!(v1, vec![super::Ele::I64(10), super::Ele::I64(20)]);

        let (t2, v2) = iter.next().unwrap();
        assert_eq!(t2, super::Ele::I64(2));
        assert_eq!(v2, vec![super::Ele::I64(30), super::Ele::I64(40)]);

        assert!(iter.next().is_none());
    }
}
