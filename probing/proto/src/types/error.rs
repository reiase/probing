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

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("deserialization error: {0}")]
    DeserializationError(String),

    #[error("protocol version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: String, got: String },

    #[error("invalid node configuration: {0}")]
    InvalidNodeConfig(String),

    #[error("node not found: {0}")]
    NodeNotFound(String),
}
