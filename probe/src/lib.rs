use std::fs;

use probe::{self, probe_command_handler};
use probe_common::cli::ProbeCommand;
use pyo3::prelude::*;

#[pyfunction]
#[pyo3(signature = (address=None, background=true, pprof=false))]
fn init(address: Option<String>, background: bool, pprof: bool) -> Result<(), std::io::Error> {
    if let Ok(_path) = fs::read_link("/proc/self/exe") {
        eprintln!("{}: loading libprob", _path.display());
    }
    let mut cmds = vec![];
    if pprof {
        cmds.push(ProbeCommand::Perf);
    }
    if background {
        cmds.push(ProbeCommand::ListenRemote { address })
    }
    for cmd in cmds {
        probe_command_handler(cmd).unwrap();
    }
    Ok(())
}

#[pymodule]
fn _probe(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    Ok(())
}
