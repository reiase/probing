use std::fs;

use probe;
use pyo3::prelude::*;

#[pyfunction]
#[pyo3(signature = (addr=None, background=false, pprof=false))]
fn init(addr: Option<String>, background: bool, pprof: bool) -> Result<(), std::io::Error> {
    if let Ok(_path) = fs::read_link("/proc/self/exe") {
        eprintln!("{}: loading libprob", _path.display());
    }
    // probe::enable_probe_server(addr, background, pprof)
    Ok(())
}

#[pymodule]
fn _probe(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    Ok(())
}
