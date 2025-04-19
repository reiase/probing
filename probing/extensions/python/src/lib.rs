#[macro_use]
extern crate ctor;

mod pkg;

pub mod extensions;
pub mod flamegraph;
pub mod pprof;
pub mod pycode;
pub mod python;
pub mod repl;

mod setup;

use std::ffi::CStr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use log::error;
use pkg::TCPStore;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyModule;
use pyo3::types::PyModuleMethods;

use probing_core::ENGINE;
use probing_proto::protocol::probe::Probe;
use probing_proto::protocol::probe::ProbeFactory;
use probing_proto::protocol::process::CallFrame;

use extensions::python::ExternalTable;
use repl::PythonRepl;

use crate::pycode::get_code;

#[derive(Default)]
pub struct PythonProbe {}

const DUMP_STACK: &CStr = c_str!(
    r#"
def _get_obj_type(obj):
    try:
        m = type(obj).__module__
        n = type(obj).__name__
        return f"{m}.{n}"
    except Exception:
        return str(type(obj))


def _get_obj_repr(obj, value=False):
    typ = _get_obj_type(obj)
    ret = {
        "id": id(obj),
        "class": _get_obj_type(obj),
    }
    if typ == "torch.Tensor":
        ret["shape"] = str(obj.shape)
        ret["dtype"] = str(obj.dtype)
        ret["device"] = str(obj.device)
    if value:
        ret["value"] = str(obj)[:150]
    return ret

stacks = []

import sys

curr = sys._getframe(1)
while curr is not None:
    stack = {"PyFrame": {
        "file": curr.f_code.co_filename,
        "func": curr.f_code.co_name,
        "lineno": curr.f_lineno,
        "locals": {
            k: _get_obj_repr(v, value=True) for k, v in curr.f_locals.items()
        },
    }}
    stacks.append(stack)
    curr = curr.f_back
import json
retval = json.dumps(stacks)
"#
);

pub static CALLSTACKS: Mutex<Option<Vec<CallFrame>>> = Mutex::new(None);

pub fn backtrace_signal_handler() {
    let frames = Python::with_gil(|py| {
        let global = PyDict::new(py);
        if let Err(err) = py.run(DUMP_STACK, Some(&global), Some(&global)) {
            error!("error extract call stacks {}", err.to_string());
            return None;
        }
        match global.get_item("retval") {
            Ok(frames) => {
                if let Some(frames) = frames {
                    frames.extract::<String>().ok()
                } else {
                    error!("error extract call stacks");
                    None
                }
            }
            Err(err) => {
                error!("error extract call stacks {}", err.to_string());
                None
            }
        }
    });

    if let Some(frames) = frames {
        match serde_json::from_str::<Vec<CallFrame>>(frames.as_str()) {
            Ok(frames) => {
                let mut callstacks = CALLSTACKS.lock().unwrap();
                *callstacks = Some(frames);
            }
            Err(err) => {
                error!("error deserializing dump stack result: {}", err.to_string());
            }
        }
    } else {
        error!("error running dump stack code");
    }
}

impl Probe for PythonProbe {
    fn backtrace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>> {
        {
            CALLSTACKS.lock().unwrap().take();
        }
        let tid = tid.unwrap_or(std::process::id() as i32);
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(tid), nix::sys::signal::SIGUSR2)?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        match CALLSTACKS.lock().unwrap().take() {
            Some(frames) => Ok(frames),
            None => Err(anyhow::anyhow!("no call stack")),
        }
    }

    fn eval(&self, code: &str) -> Result<String> {
        let code: String = code.into();
        let mut repl = PythonRepl::default();
        Ok(repl.process(code.as_str()).unwrap_or_default())
    }

    fn flamegraph(&self) -> Result<String> {
        Ok(flamegraph::flamegraph())
    }
}

#[derive(Default)]
pub struct PythonProbeFactory {}

impl ProbeFactory for PythonProbeFactory {
    fn create(&self) -> Arc<dyn Probe> {
        Arc::new(PythonProbe::default())
    }
}

#[pyfunction]
fn query_json(_py: Python, sql: String) -> PyResult<String> {
    let result = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { ENGINE.read().await.query(sql.as_str()) })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

pub fn create_probing_module() -> PyResult<()> {
    Python::with_gil(|py| -> PyResult<()> {
        let sys = PyModule::import(py, "sys")?;
        let modules = sys.getattr("modules")?;

        if !modules.contains("probing")? {
            let m = PyModule::new(py, "probing")?;
            modules.set_item("probing", m)?;
        }

        let m = PyModule::import(py, "probing")?;
        if m.hasattr(pyo3::intern!(py, "_C"))? {
            return Ok(());
        }
        m.setattr(pyo3::intern!(py, "_C"), 42)?;
        m.add_class::<ExternalTable>()?;
        m.add_class::<TCPStore>()?;
        m.add_function(wrap_pyfunction!(query_json, py)?)?;

        Ok(())
    })
}
