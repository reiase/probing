use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Unsupported option: {0}")]
    UnsupportedOption(String),

    #[error("Unsupported option value: {0}={1}")]
    UnsupportedOptionValue(String, String),
}
