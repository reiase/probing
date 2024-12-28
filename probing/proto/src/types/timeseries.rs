use crate::types::array::Array;

pub enum Chunk {
    Raw(Array),
    Compressed(Vec<u8>),
    Ref,
}

pub struct Slice {
    pub start: Option<u64>,
    pub end: Option<u64>,
    pub data: Chunk,
}

pub struct TimeSeries {
    pub slices: Vec<Slice>,
}
