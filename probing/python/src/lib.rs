pub mod flamegraph;
pub mod plugins;
pub mod pprof;
pub mod pycode;
pub mod repl;

use std::ffi::CStr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;

use log::error;
use nix::unistd::Pid;
use once_cell::sync::Lazy;
use pprof::PPROF_HOLDER;
use probing_engine::core::ConfigEntry;
use probing_engine::core::ConfigExtension;
use probing_engine::core::DataFusionError;
use probing_engine::core::ExtensionOptions;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDict;
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

    fn flamegraph(&self) -> Result<String> {
        Ok(flamegraph::flamegraph())
    }

    fn enable(&self, feture: &str) -> Result<()> {
        create_probing_module()?;
        match feture {
            "profiling" => {
                PPROF_HOLDER.setup(100);
                Ok(())
            }
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

    fn disable(&self, _feture: &str) -> anyhow::Result<()> {
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
        let sys = PyModule::import(py, "sys")?;
        let modules = sys.getattr("modules")?;

        if modules.contains("probing")? {
            return Ok(());
        }

        let m = PyModule::new(py, "probing")?;
        m.add_class::<ExternalTable>()?;

        modules.set_item("probing", m)?;

        Ok(())
    })?;
    Ok(())
}

pub static PROBING_OPTIONS: Lazy<Mutex<ProbingOptions>> =
    Lazy::new(|| Mutex::new(ProbingOptions::default()));

#[derive(Debug, Clone)]
pub struct ProbingOptions {
    pprof_sample_freq: i32,
    torch_sample_ratio: f64,
}

impl Default for ProbingOptions {
    fn default() -> Self {
        ProbingOptions {
            pprof_sample_freq: 0,
            torch_sample_ratio: 0.0,
        }
    }
}

impl ConfigExtension for ProbingOptions {
    const PREFIX: &'static str = "probing";
}

impl ExtensionOptions for ProbingOptions {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn cloned(&self) -> Box<dyn ExtensionOptions> {
        Box::new(self.clone())
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), DataFusionError> {
        log::debug!("probing update setting: {} = {}", key, value);
        let mut global_setting = PROBING_OPTIONS.lock().unwrap();
        match key {
            "pprof_sample_freq" | "pprof.sample_freq" | "pprof.sample.freq" => {
                let sample_freq: i32 = value.parse().unwrap_or(100);
                global_setting.pprof_sample_freq = sample_freq;
                PPROF_HOLDER.setup(sample_freq);
            }
            "torch_sample_ratio" | "torch.sample_ratio" | "torch.sample.ratio" => {
                let sample_ratio: f64 = value.parse().unwrap_or(0.0);
                global_setting.torch_sample_ratio = sample_ratio;
                let filename = format!("{}.py", "torch_profiling");
                let code = get_code(filename.as_str());
                match if let Some(code) = code {
                    Python::with_gil(|py| {
                        let code = format!("{}\0", code);
                        let code = CStr::from_bytes_with_nul(code.as_bytes())?;
                        py.run(code, None, None).map_err(|err| {
                            anyhow::anyhow!("error apply setting {}={}: {}", key, value, err)
                        })?;

                        let code = format!("torch_profiling({})\0", sample_ratio);
                        let code = CStr::from_bytes_with_nul(code.as_bytes())?;
                        py.run(code, None, None).map_err(|err| {
                            anyhow::anyhow!("error apply setting {}={}: {}", key, value, err)
                        })
                    })
                } else {
                    Err(anyhow::anyhow!("unsupported setting {}={}", key, value))
                } {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("{}", err);
                    }
                }
            }
            _ => println!("unknown setting {}={}", key, value),
        }
        Ok(())
    }

    fn entries(&self) -> Vec<ConfigEntry> {
        let global_setting = PROBING_OPTIONS.lock().unwrap();
        let ret = vec![
            ConfigEntry {
                key: "probing.pprof.sample_freq".to_string(),
                value: Some(format!("{}", global_setting.pprof_sample_freq)),
                description: "pprof sample frequency",
            },
            ConfigEntry {
                key: "probing.torch.sample_ratio".to_string(),
                value: Some(format!("{}", global_setting.torch_sample_ratio)),
                description: "torch profiling sample ratio",
            },
        ];
        println!("{:?}", ret);
        ret
    }
}
