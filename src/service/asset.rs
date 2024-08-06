use hyper::body::Bytes;

#[cfg(not(debug_assertions))]
use include_dir::{include_dir, Dir};

#[cfg(not(debug_assertions))]
static ASSET: Dir = include_dir!("app/dist");

#[cfg(not(debug_assertions))]
pub fn contains(path: &str) -> bool {
    ASSET.get_file(path).is_some()
}

#[cfg(not(debug_assertions))]
pub fn get(path: &str) -> Bytes {
    let content = ASSET
        .get_file(path.trim_start_matches('/'))
        .unwrap()
        .contents_utf8()
        .unwrap();
    Bytes::copy_from_slice(content.as_bytes())
}

#[cfg(debug_assertions)]
pub fn contains(path: &str) -> bool {
    let path = path.trim_start_matches('/');
    std::path::Path::new(format!("app/dist/{}", path).as_str()).exists()
}

#[cfg(debug_assertions)]
pub fn get(path: &str) -> Bytes {
    let path = format!("app/dist/{}", path.trim_start_matches('/'));
    let content = std::fs::read(path).unwrap();
    Bytes::from(content)
}
