use std::ffi::{CStr, CString};
use std::sync::Mutex;

use anyhow::Result;
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3::{types::PyDict, Python};

use crate::pycode::get_code;

pub static CRASH_HANDLER: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
pub static OLD_HANDLER: Lazy<Option<Py<PyAny>>> = Lazy::new(|| None);

fn call_default_handler(typ: Py<PyAny>, value: Py<PyAny>, traceback: Py<PyAny>) -> Result<()> {
    let code = get_code("crash_handler.py").unwrap_or_default();
    let code = format!("{code}\0");
    let code = CStr::from_bytes_with_nul(code.as_bytes())?;
    Python::with_gil(|py| -> Result<()> {
        let global = PyDict::new(py);
        py.run(code, Some(&global), None)?;
        if let Some(handler) = global.get_item("crash_handler")? {
            let args = PyTuple::new(py, [typ, value, traceback])?;
            handler.call(args, None)?;
        }
        Ok(())
    })
}

fn call_custom_handler(
    handler: &str,
    typ: Py<PyAny>,
    value: Py<PyAny>,
    traceback: Py<PyAny>,
) -> Result<()> {
    Python::with_gil(|py| -> Result<()> {
        let locals = PyDict::new(py);
        if handler.contains(',') {
            let parts: Vec<&str> = handler.split(".").collect();
            let pkg = py.import(parts[0])?;
            locals.set_item(parts[0], pkg)?;
        }
        locals.set_item("type", typ)?;
        locals.set_item("value", value)?;
        locals.set_item("traceback", traceback)?;
        let ret = (|| {
            let expr = CString::new(handler)?;
            py.eval(&expr, None, Some(&locals))
        })();

        println!("crash handler: {ret:?}");
        Ok(())
    })
}

#[pyfunction]
pub fn crash_handler(typ: Py<PyAny>, value: Py<PyAny>, traceback: Py<PyAny>) {
    log::debug!(
        "call crash handler: {:?}",
        CRASH_HANDLER.lock().unwrap().clone()
    );
    if let Some(handler) = CRASH_HANDLER.lock().unwrap().as_ref() {
        let ret = match handler.as_str() {
            "default" => call_default_handler(typ, value, traceback),
            handler => call_custom_handler(handler, typ, value, traceback),
        };
        match ret {
            Ok(_) => {}
            Err(err) => {
                log::error!("error calling crash handler: {err}");
            }
        }
    }
}

pub fn enable_crash_handler() -> anyhow::Result<()> {
    Python::with_gil(|py| -> anyhow::Result<()> {
        log::debug!("enable crash handler");
        let sys = py.import("sys")?;
        let func = wrap_pyfunction!(crash_handler, sys)?;

        let sys = py.import("sys")?;
        sys.setattr("excepthook", func)?;
        Ok(())
    })?;
    Ok(())
}

pub fn enable_monitoring(filename: &str) -> anyhow::Result<()> {
    Python::with_gil(|py| {
        let ver = py.version_info();
        if ver.major != 3 || ver.minor < 12 {
            return Err(anyhow::anyhow!("Python version must be 3.8+"));
        }

        let filename = if filename == "default" {
            "monitoring.py"
        } else {
            filename
        };

        let code = get_code(filename).unwrap_or_default();

        let code = format!("{code}\0");
        let code = CStr::from_bytes_with_nul(code.as_bytes())?;
        py.run(code, None, None)
            .map_err(|err| anyhow::anyhow!("error apply monitoring {}: {}", filename, err))?;
        Ok(())
    })
}
