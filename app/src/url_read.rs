use gloo_net::http::Request;
use leptos::prelude::*;
use probing_proto::prelude::{DataFrame, Message, Query, QueryDataFormat};
use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::errors::AppError;

pub async fn url_read_str(url: &str) -> Result<String, AppError> {
    Request::get(url)
        .send()
        .await
        .map_err(|e| AppError::NetworkError(e.to_string()))?
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
            Err(err) => Err(AppError::SerializationError(format!(
                "Failed to deserialize response: {}",
                err
            ))),
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

pub async fn read_query(query: &str) -> Result<DataFrame, AppError> {
    let request = Query {
        expr: query.to_string(),
        ..Default::default()
    };
    let request = Message::new(request);
    let request = serde_json::to_string(&request)
        .map_err(|e| AppError::SerializationError(format!("Failed to serialize request: {}", e)))?;
    let response = Request::post("/query")
        .body(request)
        .map_err(|e| AppError::NetworkError(e.to_string()))?
        .send()
        .await
        .map_err(|e| AppError::NetworkError(e.to_string()))?
        .text()
        .await
        .map_err(|e| AppError::NetworkError(e.to_string()))?;

    let response: Message<QueryDataFormat> = serde_json::from_str(response.as_str())
        .map_err(|e| AppError::SerializationError(e.to_string()))?;

    match response.payload {
        QueryDataFormat::DataFrame(data_frame) => Ok(data_frame),
        _ => Err(AppError::QueryError(
            "Bad Response: DataFrame is Expected.".to_string(),
        )),
    }
}

pub fn read_query_resource(query: &str) -> LocalResource<Result<DataFrame, AppError>> {
    let query = query.to_string();
    LocalResource::new(move || {
        let value = query.clone();
        async move {
            let query = value.clone();
            read_query(query.as_str()).await
        }
    })
}
