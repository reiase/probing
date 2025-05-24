use std::env;

use axum::http::{header, StatusCode, Uri};
use axum::response::IntoResponse;
use bytes::Bytes;
use include_dir::include_dir;
use include_dir::Dir;

static ASSET: Dir = include_dir!("app/dist");

pub fn contains(path: &str) -> bool {
    if let Ok(assets_root) = env::var("PROBING_ASSETS_ROOT") {
        let path = format!("{}/{}", assets_root, path.trim_start_matches('/'));
        std::path::Path::new(path.as_str()).exists()
    } else {
        ASSET.contains(path.trim_start_matches('/'))
    }
}

pub fn get(path: &str) -> Bytes {
    if let Ok(assets_root) = env::var("PROBING_ASSETS_ROOT") {
        let path = format!("{}/{}", assets_root, path.trim_start_matches('/'));
        let content = std::fs::read(path).unwrap_or_default();
        Bytes::from(content)
    } else {
        ASSET
            .get_file(path.trim_start_matches('/'))
            .map(|f| Bytes::copy_from_slice(f.contents()))
            .unwrap_or_default()
    }
}

/// Get the content type of a file based on its extension
fn get_content_type(path: &str) -> &'static str {
    match path {
        p if p.ends_with(".html") => "text/html",
        p if p.ends_with(".js") => "application/javascript",
        p if p.ends_with(".css") => "text/css",
        p if p.ends_with(".svg") => "image/svg+xml",
        p if p.ends_with(".wasm") => "application/wasm",
        p if p.ends_with(".json") => "application/json",
        p if p.ends_with(".png") => "image/png",
        p if p.ends_with(".jpg") || p.ends_with(".jpeg") => "image/jpeg",
        p if p.ends_with(".gif") => "image/gif",
        p if p.ends_with(".ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// Handler for index page
pub async fn index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html")],
        get("/index.html"),
    )
}

/// Handler for serving static files
pub async fn static_files(uri: Uri) -> Result<impl IntoResponse, StatusCode> {
    let path = uri.path();
    if !contains(path) {
        return Err(StatusCode::NOT_FOUND);
    }
    
    log::debug!("serving file: {}", path);
    Ok((
        [(header::CONTENT_TYPE, get_content_type(path))],
        get(path),
    ))
}
