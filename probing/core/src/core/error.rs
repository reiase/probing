use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Unsupported option: {0}")]
    UnsupportedOption(String),

    #[error("Invalid option value: {0}={1}")]
    InvalidOption(String, String),

    #[error("Read-only option: {0}")]
    ReadOnlyOption(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Call error: {0}")]
    CallError(String),

    #[error("Unsupported call")]
    UnsupportedCall,
}
