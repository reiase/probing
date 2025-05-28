use std::collections::HashMap;
use std::fmt::Display;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::sync::Mutex;

use nix::libc;
use probing_core::core::EngineCall;
use probing_core::core::EngineDatasource;
use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;
use probing_proto::prelude::CallFrame;
use pyo3::types::PyAnyMethods;
use pyo3::Python;

pub use exttbls::ExternalTable;
pub use tbls::PythonPlugin;

use crate::flamegraph;
use crate::python::enable_crash_handler;
use crate::python::enable_monitoring;
use crate::python::CRASH_HANDLER;
use crate::repl::PythonRepl;

use crate::CALLSTACK_SENDER_SLOT;

/// Define a static Mutex for the backtrace function
static BACKTRACE_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
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
    #[option(aliases = ["crash.handler"])]
    crash_handler: Maybe<String>,

    /// Path to Python monitoring handler script
    #[option()]
    monitoring: Maybe<String>,

    /// Enable Python extensions by setting `python.enabled=<extension_statement>`
    #[option()]
    enabled: PyExtList,

    /// Disable Python extension by setting `python.disabled=<extension_statement>`
    #[option()]
    disabled: Maybe<String>,
}

#[async_trait]
impl EngineCall for PythonExt {
    async fn call(
        &self,
        path: &str,
        params: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        log::debug!(
            "Python extension call - path: {}, params: {:?}, body_size: {}",
            path,
            params,
            body.len()
        );
        if path == "callstack" {
            let frames = if params.contains_key("tid") {
                let tid = params.get("tid").unwrap().parse::<i32>().unwrap();
                backtrace(Some(tid))
            } else {
                backtrace(None)
            }
            .map_err(|e| {
                log::error!("Failed to get call stack: {}", e);
                EngineError::PluginError(format!("Failed to get call stack: {}", e))
            })?;
            return serde_json::to_vec(&frames).map_err(|e| {
                log::error!("Failed to serialize call stack: {}", e);
                EngineError::PluginError(format!("Failed to serialize call stack: {}", e))
            });
        }
        if path == "eval" {
            let code = String::from_utf8(body.to_vec()).map_err(|e| {
                log::error!("Failed to convert body to UTF-8 string: {}", e);
                EngineError::PluginError(format!("Failed to convert body to UTF-8 string: {}", e))
            })?;

            log::debug!("Python eval code: {}", code);

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
                Self::OPTION_CRASH_HANDLER.to_string(),
            )),
            Maybe::Nothing => match &crash_handler {
                Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                    Self::OPTION_CRASH_HANDLER.to_string(),
                    crash_handler.clone().into(),
                )),
                Maybe::Just(handler) => {
                    self.crash_handler = crash_handler.clone();
                    CRASH_HANDLER.lock().unwrap().replace(handler.to_string());
                    match enable_crash_handler() {
                        Ok(_) => {
                            log::info!("Python crash handler enabled: {}", handler);
                            Ok(())
                        }
                        Err(e) => {
                            log::error!("Failed to enable crash handler '{}': {}", handler, e);
                            Err(EngineError::InvalidOptionValue(
                                Self::OPTION_CRASH_HANDLER.to_string(),
                                handler.to_string(),
                            ))
                        }
                    }
                }
            },
        }
    }

    /// Set up Python monitoring
    fn set_monitoring(&mut self, monitoring: Maybe<String>) -> Result<(), EngineError> {
        log::debug!("Setting Python monitoring: {}", monitoring);
        match self.monitoring {
            Maybe::Just(_) => Err(EngineError::ReadOnlyOption(
                Self::OPTION_MONITORING.to_string(),
            )),
            Maybe::Nothing => match &monitoring {
                Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                    Self::OPTION_MONITORING.to_string(),
                    monitoring.clone().into(),
                )),
                Maybe::Just(handler) => {
                    self.monitoring = monitoring.clone();
                    match enable_monitoring(handler) {
                        Ok(_) => {
                            log::info!("Python monitoring enabled: {}", handler);
                            Ok(())
                        }
                        Err(e) => {
                            log::error!("Failed to enable monitoring '{}': {}", handler, e);
                            Err(EngineError::InvalidOptionValue(
                                Self::OPTION_MONITORING.to_string(),
                                handler.to_string(),
                            ))
                        }
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
                    Self::OPTION_ENABLED.to_string(),
                    enabled.clone().into(),
                ));
            }
            Maybe::Just(e) => e,
        };

        // Check if extension is already loaded
        if self.enabled.0.contains_key(ext) {
            return Err(EngineError::PluginError(format!(
                "Python extension '{}' is already enabled",
                ext
            )));
        }

        // Execute Python code and get the extension object
        let pyext = execute_python_code(ext)
            .map_err(|e| EngineError::InvalidOptionValue(Self::OPTION_ENABLED.to_string(), e))?;

        // Store the extension
        self.enabled.0.insert(ext.clone(), pyext);
        log::info!("Python extension enabled: {}", ext);
        log::debug!("Current enabled extensions: {}", self.enabled);

        Ok(())
    }

    /// Disable a previously enabled Python extension
    fn set_disabled(&mut self, disabled: Maybe<String>) -> Result<(), EngineError> {
        // Extract extension name from Maybe
        let ext = match &disabled {
            Maybe::Nothing => {
                return Err(EngineError::InvalidOptionValue(
                    Self::OPTION_DISABLED.to_string(),
                    disabled.clone().into(),
                ));
            }
            Maybe::Just(e) => e,
        };

        // Remove extension if it exists
        if let Some(pyext) = self.enabled.0.remove(ext) {
            log::info!("Disabling Python extension: {}", ext);

            // Call deinit method on extension object
            Python::with_gil(|py| {
                // Call the Python object's deinit method
                match pyext.call_method0(py, "deinit") {
                    Ok(_) => {
                        log::debug!("Extension '{}' deinitialized successfully", ext);
                        Ok(())
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to call deinit method on '{}': {}", ext, e);
                        log::error!("{}", error_msg);
                        Err(EngineError::PluginError(error_msg))
                    }
                }
            })
        } else {
            log::debug!(
                "Python extension '{}' was not enabled, nothing to disable",
                ext
            );
            // Extension wasn't found, not an error
            Ok(())
        }
    }
}

/// Execute Python code and return the resulting object
/// The code should return an object with init/deinit methods
pub fn execute_python_code(code: &str) -> Result<pyo3::Py<pyo3::PyAny>, String> {
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

        log::info!("Python extension loaded successfully: {}", code);
        Ok(result.unbind())
    })
}

fn backtrace(tid: Option<i32>) -> Result<Vec<CallFrame>> {
    // Acquire the lock at the beginning of the function
    let _guard = BACKTRACE_MUTEX.try_lock().map_err(|e| {
        log::error!("Failed to acquire BACKTRACE_MUTEX: {}", e);
        anyhow::anyhow!("Failed to acquire backtrace lock: {}", e)
    })?;

    let (tx, rx) = mpsc::channel::<Vec<CallFrame>>();
    match CALLSTACK_SENDER_SLOT.try_lock() {
        Ok(mut sender_slot) => sender_slot.replace(tx),
        Err(err) => {
            log::error!("Failed to lock CALLSTACK_SENDER_SLOT: {}", err);
            return Err(anyhow::anyhow!("Failed to lock call stack sender slot"));
        }
    };

    let pid = tid
        .map(|t| nix::unistd::Pid::from_raw(t))
        .unwrap_or_else(|| nix::unistd::getpid());

    let ret = unsafe {
        let pid = libc::getpid();
        let tid = tid.unwrap_or_else(|| libc::getpid());
        log::debug!("Sending SIGUSR2 signal to process {} (tid: {:?})", pid, tid);

        libc::syscall(libc::SYS_tgkill, pid, tid, libc::SIGUSR2)
    };
    if ret != 0 {
        log::error!("Failed to send SIGUSR2 to process {pid}(tid {:?})", tid);

        return Err(anyhow::anyhow!(
            "Failed to send signal to process {pid}(tid {:?})",
            tid
        ));
    }

    let frames_result = match rx.recv_timeout(Duration::from_secs(5)) {
        // Increased timeout slightly
        Ok(frames) => Ok(frames),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            log::warn!("Timeout waiting for call stack from signal handler after SIGUSR2");
            Err(anyhow::anyhow!("No call stack received: timeout"))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            log::error!("Call stack channel disconnected. Signal handler might have failed or sender was not properly used.");
            Err(anyhow::anyhow!(
                "No call stack received: channel disconnected"
            ))
        }
    };

    frames_result
}
