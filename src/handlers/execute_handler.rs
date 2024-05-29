use pyo3::Python;
use std::fs;

pub fn execute_handler(code_or_path: String) {
    let path = std::path::Path::new(&code_or_path);
    if path.is_file() {
        Python::with_gil(|py| {
            let contents =
                fs::read_to_string(path).expect("Should have been able to read the file");
            let _ = py.run_bound(&contents, None, None);
        })
    } else {
        Python::with_gil(|py| {
            let _ = py.run_bound(&code_or_path, None, None);
        })
    }
}
