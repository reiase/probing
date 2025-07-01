use super::config::{get_max_file_size, ALLOWED_FILE_DIRS};
use super::error::ApiResult;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Validate that the requested path is safe and within allowed directories
fn validate_path(path: &str) -> Result<PathBuf, String> {
    // Reject empty paths
    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // Reject paths with null bytes (security risk)
    if path.contains('\0') {
        return Err("Path contains invalid characters".to_string());
    }

    // Convert to canonical path to resolve any .. or . components
    let requested_path = Path::new(path);
    let canonical_path = match requested_path.canonicalize() {
        Ok(path) => path,
        Err(_) => return Err("Invalid or non-existent path".to_string()),
    };

    // Check if the canonical path is within any allowed base directory
    let mut is_allowed = false;
    for base_dir in ALLOWED_FILE_DIRS {
        let base_path = match Path::new(base_dir).canonicalize() {
            Ok(path) => path,
            Err(_) => continue, // Skip non-existent base directories
        };

        if canonical_path.starts_with(&base_path) {
            is_allowed = true;
            break;
        }
    }

    if !is_allowed {
        return Err("Access denied: path is outside allowed directories".to_string());
    }

    Ok(canonical_path)
}

/// Read a file from the filesystem with security checks
pub async fn read_file(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> ApiResult<String> {
    let path = params
        .get("path")
        .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

    // Validate the path
    let safe_path = validate_path(path).map_err(|e| {
        log::warn!("Path validation failed for '{path}': {e}");
        anyhow::anyhow!("Invalid path: {}", e)
    })?;

    // Check file size before reading
    let metadata = tokio::fs::metadata(&safe_path).await.map_err(|e| {
        log::warn!("Failed to get metadata for {safe_path:?}: {e}");
        anyhow::anyhow!("Cannot access file")
    })?;

    let max_file_size = get_max_file_size();
    if metadata.len() > max_file_size {
        return Err(anyhow::anyhow!("File too large (max {} bytes allowed)", max_file_size).into());
    }

    // Read file content asynchronously
    let content = tokio::fs::read_to_string(&safe_path).await.map_err(|e| {
        log::warn!("Failed to read file {safe_path:?}: {e}");
        anyhow::anyhow!("Cannot read file")
    })?;

    log::info!("Successfully read file: {safe_path:?}");
    Ok(content)
}
