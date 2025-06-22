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

use std::sync::mpsc;
use std::sync::Mutex;

use log::error;
use once_cell::sync::Lazy;
use pkg::TCPStore;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::types::PyModuleMethods;

use probing_core::ENGINE;
use probing_proto::prelude::CallFrame;

pub static NATIVE_CALLSTACK_SENDER_SLOT: Lazy<Mutex<Option<mpsc::Sender<Vec<CallFrame>>>>> =
    Lazy::new(|| Mutex::new(None));

use cpp_demangle::Symbol;

fn get_native_stacks() -> Option<Vec<CallFrame>> {
    let mut frames = vec![];
    backtrace::trace(|frame| {
        let ip = frame.ip();
        let symbol_address = frame.symbol_address(); // Keep as *mut c_void for formatting
        backtrace::resolve_frame(frame, |symbol| {
            let func_name = symbol
                .name()
                .and_then(|name| name.as_str())
                .map(|raw_name| {
                    Symbol::new(raw_name)
                        .ok()
                        .map(|demangled| demangled.to_string())
                        .unwrap_or_else(|| raw_name.to_string())
                })
                .unwrap_or_else(|| format!("unknown@{:p}", symbol_address));

            let file_name = symbol
                .filename()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();

            frames.push(CallFrame::CFrame {
                ip: format!("{:p}", ip),
                file: file_name,
                func: func_name,
                lineno: symbol.lineno().unwrap_or(0) as i64,
            });
        });
        true
    });
    Some(frames)
}

// Helper function to attempt sending frames, returns true on success, false on failure.
fn try_send_native_frames_to_channel(frames: Vec<CallFrame>, context_msg: &str) -> bool {
    match NATIVE_CALLSTACK_SENDER_SLOT.try_lock() {
        Ok(guard) => {
            if let Some(sender) = guard.as_ref() {
                if sender.send(frames).is_ok() {
                    true
                } else {
                    error!("Failed to send frames for {} via channel.", context_msg);
                    false
                }
            } else {
                log::trace!("No active callstack sender found for {}.", context_msg);
                true
            }
        }
        Err(e) => {
            error!(
                "Failed to lock NATIVE_CALLSTACK_SENDER_SLOT for {}: {}",
                context_msg, e
            );
            false
        }
    }
}

pub fn backtrace_signal_handler() {
    let native_stacks = get_native_stacks().unwrap_or_default();

    if !try_send_native_frames_to_channel(native_stacks, "native stacks (initial send)") {
        error!("Signal handler: CRITICAL - Failed to send native stacks. Receiver might timeout or get incomplete data.");
        return;
    }
}

#[pyfunction]
fn query_json(_py: Python, sql: String) -> PyResult<String> {
    let result = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { ENGINE.read().await.async_query(sql.as_str()).await })
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
        m.add_class::<crate::extensions::python::ExternalTable>()?;
        m.add_class::<TCPStore>()?;
        m.add_function(wrap_pyfunction!(query_json, py)?)?;

        Ok(())
    })
}
