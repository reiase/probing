use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Unsupported option: {0}")]
    UnsupportedOption(String),

    #[error("Invalid option value: {0}={1}")]
    InvalidOption(String, String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
