use anyhow::Result;
use pyo3::{types::PyAnyMethods, Python};

pub fn show_traceable_functions(filter: Option<String>) -> Result<String> {
    Python::with_gil(|py| {
        let pi = py.import_bound("probing.trace")?;
        // let pi = pi.getattr("trace")?;
        let ret = pi.getattr("list_traceable_functions")?.call1(
            (filter,)
        )?;
        Ok(ret.to_string())
    })
}