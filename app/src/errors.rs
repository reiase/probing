use thiserror::Error;
use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize)]
#[derive(Clone, Debug, Error)]
pub enum AppError {
    #[error("Page Not Found")]
    NotFound,

    #[error("TimeOut")]
    TimeOut,

    #[error("HTTP Error")]
    HttpError(String),
}