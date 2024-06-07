use bytes::Bytes;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "dist"]
struct Asset;

pub fn contains(path: &str) -> bool {
    Asset::get(path.trim_start_matches('/')).is_some()
}

pub fn get(path: &str) -> Bytes {
    let content = Asset::get(path.trim_start_matches('/')).unwrap();
    Bytes::copy_from_slice(content.data.as_ref())
}
