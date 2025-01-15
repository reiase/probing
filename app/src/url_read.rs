use gloo_net::http::Request;
use leptos::prelude::*;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::errors::AppError;

pub async fn url_read_str(url: &str) -> Result<String, AppError> {
    Request::get(url)
        .send()
        .await
        .map_err(|e| AppError::HttpError(e.to_string()))?
        .text()
        .await
        .map_err(|_| AppError::HttpError("Bad Response: String is Expected.".to_string()))
}

pub async fn url_read<T: DeserializeOwned>(url: &str) -> Result<T, AppError> {
    match url_read_str(url)
        .await
        .map(|s| serde_json::from_str::<T>(s.as_str()))
    {
        Ok(val) => match val {
            Ok(t) => Ok(t),
            Err(err) => Err(AppError::HttpError(
                format!("Bad Response: {}", err).to_string(),
            )),
        },
        Err(e) => Err(e),
    }
}

pub fn url_read_resource<T: Serialize + DeserializeOwned + 'static>(
    url: &str,
) -> LocalResource<Result<T, AppError>> {
    let url = url.to_string();
    LocalResource::new(move || {
        let value = url.clone();
        async move {
            let url = value.clone();
            url_read(url.as_str()).await
        }
    })
}
