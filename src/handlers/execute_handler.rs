use anyhow::Context;
use anyhow::Result;
use pyo3::Python;
use std::fs;

pub fn execute_handler(code_or_path: String) -> Result<()> {
    let path = std::path::Path::new(&code_or_path);
    if path.is_file() {
        Python::with_gil(|py| {
            let contents =
                fs::read_to_string(path).expect("Should have been able to read the file");
            py.run_bound(&contents, None, None)
                .with_context(|| format!("failed to execute script: {}", path.display()))
        })
    } else {
        Python::with_gil(|py| {
            py.run_bound(&code_or_path, None, None)
                .with_context(|| format!("failed to execute script: {}", code_or_path))
        })
    }
}
