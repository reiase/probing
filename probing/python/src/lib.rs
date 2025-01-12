pub mod plugins;
pub mod pycode;
pub mod repl;

use std::ffi::CStr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;

use log::error;
use nix::unistd::Pid;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::types::PyModuleMethods;

use probing_proto::protocol::probe::Probe;
use probing_proto::protocol::probe::ProbeFactory;
use probing_proto::protocol::process::CallFrame;

use plugins::external_tables::ExternalTable;
use repl::PythonRepl;

use crate::pycode::get_code;

#[derive(Default)]
pub struct PythonProbe {}

const DUMP_STACK: &CStr = c_str!(
    r#"
stacks = []

import sys

curr = sys._getframe(2)
while curr is not None:
    stack = {
        "file": curr.f_code.co_filename,
        "func": curr.f_code.co_name,
        "lineno": curr.f_lineno,
        "locals": {
            k: _get_obj_repr(v, value=True) for k, v in curr.f_locals.items()
        },
    }
    stacks.append(stack)
    curr = curr.f_back
import json
json.dumps(stacks)
"#
);

pub static CALLSTACKS: Mutex<Option<Vec<CallFrame>>> = Mutex::new(None);

pub fn backtrace_signal_handler() {
    let frames = Python::with_gil(|py| match py.eval(DUMP_STACK, None, None) {
        Ok(frames) => Ok(frames.to_string()),
        Err(err) => Err(anyhow::anyhow!(
            "error extract call stacks {}",
            err.to_string()
        )),
    });
    if let Ok(frames) = frames {
        let frames = serde_json::from_str::<Vec<CallFrame>>(frames.as_str());
        if let Ok(frames) = frames {
            let mut callstacks = CALLSTACKS.lock().unwrap();
            *callstacks = Some(frames);
        } else {
            error!("error deserializing dump stack result");
        }
    } else {
        error!("error running dump stack code");
    }
}

impl Probe for PythonProbe {
    fn backtrace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>> {
        // let frames = Python::with_gil(|py| match py.eval(DUMP_STACK, None, None) {
        //     Ok(frames) => Ok(frames.to_string()),
        //     Err(err) => Err(anyhow::anyhow!(
        //         "error extract call stacks {}",
        //         err.to_string()
        //     )),
        // })?;
        // serde_json::from_str::<Vec<CallFrame>>(frames.as_str())
        //     .with_context(|| "error deserializing dump stack result".to_string())
        {
            CALLSTACKS.lock().unwrap().take();
        }
        let tid = tid.unwrap_or(std::process::id() as i32);
        nix::sys::signal::kill(Pid::from_raw(tid), nix::sys::signal::SIGUSR2)?;
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

    fn enable(&self, feture: &str) -> Result<()> {
        match feture {
            "profiling" => Ok(()),
            name => {
                let filename = if let Some(pos) = name.find('(') {
                    &name[..pos]
                } else {
                    name
                };

                let filename = format!("{}.py", filename);
                let code = get_code(filename.as_str());
                if let Some(code) = code {
                    Python::with_gil(|py| {
                        let code = format!("{}\0", code);
                        let code = CStr::from_bytes_with_nul(code.as_bytes())?;
                        py.run(code, None, None).map_err(|err| {
                            anyhow::anyhow!("error loading feature {}: {}", name, err)
                        })?;

                        let code = format!("{}\0", name);
                        let code = CStr::from_bytes_with_nul(code.as_bytes())?;
                        py.run(code, None, None).map_err(|err| {
                            anyhow::anyhow!("error running feature {}: {}", name, err)
                        })
                    })
                } else {
                    Err(anyhow::anyhow!("unsupported feature {}", name))
                }
            }
        }
    }

    fn disable(&self, feture: &str) -> anyhow::Result<()> {
        todo!()
    }
}

#[derive(Default)]
pub struct PythonProbeFactory {}

impl ProbeFactory for PythonProbeFactory {
    fn create(&self) -> Arc<dyn Probe> {
        Arc::new(PythonProbe::default())
    }
}

pub fn create_probing_module() -> PyResult<()> {
    Python::with_gil(|py| -> PyResult<()> {
        let m = PyModule::new(py, "probing")?;
        m.add_class::<ExternalTable>()?;

        let sys = PyModule::import(py, "sys")?;
        let modules = sys.getattr("modules")?;
        modules.set_item("probing", m)?;

        Ok(())
    })?;
    Ok(())
}
