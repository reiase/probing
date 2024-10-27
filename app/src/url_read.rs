use gloo_net::http::Request;
use leptos::{create_resource, Resource};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::errors::AppError;

pub async fn url_read_str(url: &str) -> Result<String, AppError> {
    Ok(Request::get(url)
        .send()
        .await
        .map_err(|e| AppError::HttpError(e.to_string()))?
        .text()
        .await
        .map_err(|_| AppError::HttpError("Bad Response: String is Expected.".to_string()))?)
}

pub async fn url_read<T: DeserializeOwned>(url: &str) -> Result<T, AppError> {
    match url_read_str(url)
        .await
        .map(|s| serde_json::from_str::<T>(s.as_str()))
    {
        Ok(val) => match val {
            Ok(t) => Ok(t),
            Err(_) => Err(AppError::HttpError(
                "Bad Resonse: Unable to Decode".to_string(),
            )),
        },
        Err(e) => Err(e),
    }
}

pub fn url_read_resource<T: Serialize + DeserializeOwned>(
    url: &str,
) -> Resource<(), Result<T, AppError>> {
    let url = url.to_string();
    create_resource(
        move || {},
        move |_| {
            let value = url.clone();
            async move {
                let url = value.clone();
                url_read(url.as_str()).await
            }
        },
    )
}
