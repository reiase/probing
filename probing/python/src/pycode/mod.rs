use include_dir::{include_dir, Dir};

static CODE: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/pycode/");

pub(crate) fn get_code(path: &str) -> Option<String> {
    if let Ok(code_root) = std::env::var("PROBING_CODE_ROOT") {
        let path = format!("{}/{}", code_root, path.trim_start_matches('/'));
        std::fs::read_to_string(path).ok()
    } else {
        CODE.get_file(path.trim_start_matches('/'))
            .map(|f| f.contents_utf8().unwrap_or_default().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_code() {
        let code = get_code("torch_profiling.py");
        assert!(code.is_some());
    }
}
