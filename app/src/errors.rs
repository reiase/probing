use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize, Deserialize, Clone, Debug, Error)]
pub enum AppError {
    #[error("Page Not Found")]
    NotFound,

    #[error("Timeout Error")]
    Timeout,

    #[error("HTTP Error: {0}")]
    HttpError(String),

    #[error("Serialization Error: {0}")]
    SerializationError(String),

    #[error("Network Error: {0}")]
    NetworkError(String),

    #[error("Query Error: {0}")]
    QueryError(String),

    #[error("Data Processing Error: {0}")]
    DataProcessingError(String),
}
