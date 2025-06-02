use include_dir::{include_dir, Dir};
use std::path::Path;

static CODE: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/pycode/");

pub(crate) fn get_code(path: &str) -> Option<String> {
    let clean_path = path.trim_start_matches('/');

    std::env::var("PROBING_CODE_ROOT")
        .ok()
        .and_then(|code_root| std::fs::read_to_string(Path::new(&code_root).join(clean_path)).ok())
        .or_else(|| {
            CODE.get_file(clean_path)
                .and_then(|f| f.contents_utf8().map(|s| s.to_string()))
        })
        .or_else(|| std::fs::read_to_string(Path::new(clean_path)).ok())

    // if let Ok(code_root) = std::env::var("PROBING_CODE_ROOT") {
    //     let fs_path = Path::new(&code_root).join(clean_path);
    //     if let Ok(content) = std::fs::read_to_string(&fs_path) {
    //         return Some(content);
    //     }
    // } else {
    //     if let Some(f) = CODE.get_file(clean_path) {
    //         if let Some(content) = f.contents_utf8() {
    //             return Some(content.to_string());
    //         }
    //     }
    // }

    // let current_dir_path = Path::new(clean_path);
    // if let Ok(content) = std::fs::read_to_string(current_dir_path) {
    //     return Some(content);
    // }
    // None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_code() {
        // Test embedded code
        env::remove_var("PROBING_CODE_ROOT");
        assert!(get_code("debug_console.py").is_some());
        assert!(get_code("non_existent.py").is_none());

        // Test filesystem code
        env::set_var("PROBING_CODE_ROOT", "src/pycode");
        assert!(get_code("debug_console.py").is_some());
    }
}
