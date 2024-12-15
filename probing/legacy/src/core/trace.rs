use anyhow::Result;
use pyo3::{types::PyAnyMethods, Python};

pub fn show_traceable(filter: Option<String>) -> Result<String> {
    Python::with_gil(|py| {
        let ret = py
            .import("probing.trace")?
            .getattr("list_traceable")?
            .call1((filter,))?
            .to_string();
        Ok(ret)
    })
}
