use anyhow::Result;

use probing_dpp::cli::TraceCommand;
use pyo3::{types::PyAnyMethods, Python};

pub fn handle(cmd: TraceCommand) -> Result<String> {
    match cmd {
        TraceCommand::Python { function, watch } => Python::with_gil(|py| {
            let pi = py.import_bound("probing.trace")?;
            let _ = pi
                .getattr("trace")?
                .call1((function, watch.split(',').collect::<Vec<_>>()))?;
            Ok(Default::default())
        }),
        TraceCommand::Clear { function } => Python::with_gil(|py| {
            let pi = py.import_bound("probing.trace")?;
            let _ = pi.getattr("untrace")?.call1((function,))?;
            Ok(Default::default())
        }),
        TraceCommand::Show => Python::with_gil(|py| {
            let pi = py.import_bound("probing.trace")?;
            let ret = pi.getattr("show_trace")?.call0()?;
            Ok(ret.to_string())
        }),
    }
}
