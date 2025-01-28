use thiserror::Error;

use super::EleType;

#[derive(Error, Debug)]
pub enum ProtoError {
    #[error("wrong element type")]
    WrongElementType,

    #[error("wrong sequence type")]
    WrongSequenceType,

    #[error("type mismatch")]
    TypeMismatch { expected: EleType, got: EleType },

    #[error("invalid data type for value")]
    InvalidValueDateType,

    #[error("raw page type expected")]
    RawPageTypeExpected,

    #[error("Series capacity exceeded")]
    CapacityExceeded,

    #[error("error compress data")]
    CompressError(String),
}
