use include_dir::{include_dir, Dir};
use log::error;
use std::path::Path;

static CODE: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/pycode/");

pub(crate) fn get_code(path: &str) -> Option<String> {
    let clean_path = path.trim_start_matches('/');

    if let Ok(code_root) = std::env::var("PROBING_CODE_ROOT") {
        let fs_path = Path::new(&code_root).join(clean_path);
        match std::fs::read_to_string(&fs_path) {
            Ok(content) => Some(content),
            Err(e) => {
                error!("Failed to read file from disk {}: {}", fs_path.display(), e);
                None
            }
        }
    } else {
        CODE.get_file(clean_path).map(|f| {
            f.contents_utf8()
                .unwrap_or_else(|| {
                    error!("Embedded file {} is not valid UTF-8", clean_path);
                    ""
                })
                .to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_code() {
        // Test embedded code
        env::remove_var("PROBING_CODE_ROOT");
        assert!(get_code("torch_profiling.py").is_some());
        assert!(get_code("debug_console.py").is_some());
        assert!(get_code("non_existent.py").is_none());

        // Test filesystem code
        env::set_var("PROBING_CODE_ROOT", "src/pycode");
        assert!(get_code("debug_console.py").is_some());
    }
}
