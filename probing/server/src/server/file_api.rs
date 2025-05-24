use std::collections::HashMap;
use super::error::ApiResult;

/// Read a file from the filesystem
pub async fn read_file(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> ApiResult<String> {
    let path = params.get("path");
    if let Some(path) = path {
        let content = std::fs::read_to_string(path)?;
        Ok(content)
    } else {
        Ok("".to_string())
    }
}
