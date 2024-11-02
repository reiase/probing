use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize, Deserialize, Clone, Debug, Error)]
pub enum AppError {
    #[error("Page Not Found")]
    NotFound,

    #[error("TimeOut")]
    TimeOut,

    #[error("HTTP Error")]
    HttpError(String),
}
