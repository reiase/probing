use std::env;

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
