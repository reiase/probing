use std::collections::HashMap;
use std::fmt::Display;

pub use exttbls::ExternalTable;
use probing_core::core::EngineCall;
use probing_core::core::EngineDatasource;
use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;
use pyo3::types::PyAnyMethods;
use pyo3::Python;
pub use tbls::PythonPlugin;

use crate::flamegraph;
use crate::python::enable_crash_handler;
use crate::python::enable_monitoring;
use crate::python::CRASH_HANDLER;
use crate::repl::PythonRepl;

mod exttbls;
mod tbls;

pub use tbls::PythonNamespace;

/// Collection of Python extensions loaded into the system
#[derive(Debug, Default)]
struct PyExtList(HashMap<String, pyo3::Py<pyo3::PyAny>>);

impl Display for PyExtList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for ext in self.0.keys() {
            if first {
                write!(f, "{}", ext)?;
                first = false;
            } else {
                write!(f, ", {}", ext)?;
            }
        }
        Ok(())
    }
}

/// Python integration with the probing system
#[derive(Debug, Default, EngineExtension)]
pub struct PythonExt {
    /// Path to Python crash handler script (executed when interpreter crashes)
    #[option(name="python.crash_handler", aliases=["python.crash.handler"])]
    crash_handler: Maybe<String>,

    /// Path to Python Monitoring Handler
    #[option(name = "python.monitoring")]
    monitoring: Maybe<String>,

    /// List of enabled Python extensions, enable additional Python extensions by setting `python.enabled=<extension_statement>`
    #[option(name = "python.enabled")]
    enabled: PyExtList,

    /// Disable Python extension by setting `python.disabled=<extension_statement>`
    #[option(name = "python.disabled")]
    disabled: Maybe<String>,
}

impl EngineCall for PythonExt {
    fn call(
        &self,
        path: &str,
        params: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        println!(
            "PythonExt::call: path = {}, params = {:?}, body = {:?}",
            path, params, body
        );
        if path == "callstack" {
            let frames = if params.contains_key("tid") {
                let tid = params.get("tid").unwrap().parse::<i32>().unwrap();
                backtrace(Some(tid))
            } else {
                backtrace(None)
            }
            .map_err(|e| {
                log::error!("error getting call stack: {}", e);
                EngineError::PluginError(format!("error getting call stack: {}", e))
            })?;
            return serde_json::to_vec(&frames).map_err(|e| {
                log::error!("error serializing call stack: {}", e);
                EngineError::PluginError(format!("error serializing call stack: {}", e))
            });
        }
        if path == "eval" {
            let code = String::from_utf8(body.to_vec()).map_err(|e| {
                log::error!("error converting body to string: {}", e);
                EngineError::PluginError(format!("error converting body to string: {}", e))
            })?;

            log::debug!("PythonExt::call: eval code = {}", code);

            let mut repl = PythonRepl::default();
            return Ok(repl.process(code.as_str()).unwrap_or_default().into_bytes());
        }
        if path == "flamegraph" {
            return Ok(flamegraph::flamegraph().into_bytes());
        }
        Ok("".as_bytes().to_vec())
    }
}

impl EngineDatasource for PythonExt {
    /// Create a plugin instance for the specified namespace
    fn datasrc(
        &self,
        namespace: &str,
        _name: Option<&str>,
    ) -> Option<std::sync::Arc<dyn probing_core::core::Plugin + Sync + Send>> {
        Some(PythonPlugin::create(namespace))
    }
}

impl PythonExt {
    /// Set up a Python crash handler
    fn set_crash_handler(&mut self, crash_handler: Maybe<String>) -> Result<(), EngineError> {
        match self.crash_handler {
            Maybe::Just(_) => Err(EngineError::ReadOnlyOption(
                "python.crash_handler".to_string(),
            )),
            Maybe::Nothing => match &crash_handler {
                Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                    "python.crash_handler".to_string(),
                    crash_handler.clone().into(),
                )),
                Maybe::Just(handler) => {
                    self.crash_handler = crash_handler.clone();
                    CRASH_HANDLER.lock().unwrap().replace(handler.to_string());
                    match enable_crash_handler() {
                        Ok(_) => Ok(()),
                        Err(_) => Err(EngineError::InvalidOptionValue(
                            "python.crash_handler".to_string(),
                            handler.to_string(),
                        )),
                    }
                }
            },
        }
    }

    /// Set up Python monitoring
    fn set_monitoring(&mut self, monitoring: Maybe<String>) -> Result<(), EngineError> {
        log::debug!("setting python.monitoring = {}", monitoring);
        match self.monitoring {
            Maybe::Just(_) => Err(EngineError::ReadOnlyOption("python.monitoring".to_string())),
            Maybe::Nothing => match &monitoring {
                Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                    "python.monitoring".to_string(),
                    monitoring.clone().into(),
                )),
                Maybe::Just(handler) => {
                    self.monitoring = monitoring.clone();
                    match enable_monitoring(handler) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(EngineError::InvalidOptionValue(
                            "python.monitoring".to_string(),
                            handler.to_string(),
                        )),
                    }
                }
            },
        }
    }

    /// Enable a Python extension from code string
    fn set_enabled(&mut self, enabled: Maybe<String>) -> Result<(), EngineError> {
        // Extract extension code from Maybe
        let ext = match &enabled {
            Maybe::Nothing => {
                return Err(EngineError::InvalidOptionValue(
                    "python.enabled".to_string(),
                    enabled.clone().into(),
                ));
            }
            Maybe::Just(e) => e,
        };

        // Check if extension is already loaded
        if self.enabled.0.contains_key(ext) {
            return Err(EngineError::PluginError(format!(
                "Python extension {} already loaded",
                ext
            )));
        }

        // Execute Python code and get the extension object
        let pyext = execute_python_code(ext)
            .map_err(|e| EngineError::InvalidOptionValue("python.enabled".to_string(), e))?;

        // Store the extension
        self.enabled.0.insert(ext.clone(), pyext);
        log::debug!("setting python.enabled = {}", self.enabled);

        Ok(())
    }

    /// Disable a previously enabled Python extension
    fn set_disabled(&mut self, disabled: Maybe<String>) -> Result<(), EngineError> {
        // Extract extension name from Maybe
        let ext = match &disabled {
            Maybe::Nothing => {
                return Err(EngineError::InvalidOptionValue(
                    "python.disabled".to_string(),
                    disabled.clone().into(),
                ));
            }
            Maybe::Just(e) => e,
        };

        // Remove extension if it exists
        if let Some(pyext) = self.enabled.0.remove(ext) {
            log::debug!("removing python extension {}", ext);

            // Call deinit method on extension object
            Python::with_gil(|py| {
                // Call the Python object's deinit method
                match pyext.call_method0(py, "deinit") {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        let error_msg = format!("error calling `deinit` method: {}", e);
                        Err(EngineError::PluginError(error_msg))
                    }
                }
            })
        } else {
            // Extension wasn't found, not an error
            Ok(())
        }
    }
}

/// Execute Python code and return the resulting object
/// The code should return an object with init/deinit methods
fn execute_python_code(code: &str) -> Result<pyo3::Py<pyo3::PyAny>, String> {
    Python::with_gil(|py| {
        let pkg = py.import("probing");

        if pkg.is_err() {
            return Err(format!("Python import error: {}", pkg.err().unwrap()));
        }

        let result = pkg
            .unwrap()
            .call_method1("load_extension", (code,))
            .map_err(|e| format!("Error loading Python plugin: {}", e))?;

        // Verify the object has an init method
        if !result
            .hasattr("init")
            .map_err(|e| format!("Unable to check `init` method: {}", e))?
        {
            return Err("Plugin must have an `init` method".to_string());
        }

        // Initialize the plugin
        result
            .call_method0("init")
            .map_err(|e| format!("Error calling `init` method: {}", e))?;

        log::info!("Successfully loaded Python plugin: {}", code);
        Ok(result.unbind())
    })
}

use crate::CALLSTACKS;
use anyhow::Result;
use probing_proto::protocol::process::CallFrame;

fn backtrace(tid: Option<i32>) -> Result<Vec<CallFrame>> {
    {
        CALLSTACKS.lock().unwrap().take();
    }
    let tid = tid.unwrap_or(std::process::id() as i32);
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(tid), nix::sys::signal::SIGUSR2).map_err(
        |e| {
            log::error!("error sending signal to process {}: {}", tid, e);
            anyhow::anyhow!("error sending signal to process {}: {}", tid, e)
        },
    )?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    match CALLSTACKS.lock().unwrap().take() {
        Some(frames) => Ok(frames),
        None => Err(anyhow::anyhow!("no call stack")),
    }
}
