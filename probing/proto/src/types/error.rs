use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtoError {
    #[error("wrong element type")]
    WrongElementType,

    #[error("wrong sequence type")]
    WrongSequenceType,
}
