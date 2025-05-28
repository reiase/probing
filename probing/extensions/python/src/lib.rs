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

use std::collections::HashSet;
use std::ffi::CStr;
use std::sync::mpsc;
use std::sync::Mutex;

use lazy_static::lazy_static;
use log::error;
use once_cell::sync::Lazy;
use pkg::TCPStore;
use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyModule;
use pyo3::types::PyModuleMethods;

use probing_core::ENGINE;
use probing_proto::prelude::CallFrame;

pub static CALLSTACK_SENDER_SLOT: Lazy<Mutex<Option<mpsc::Sender<Vec<CallFrame>>>>> =
    Lazy::new(|| Mutex::new(None));

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

try:
    curr = sys._getframe(1)
except:
    curr = None
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

unsafe extern "C" {
    pub unsafe fn PyThreadState_GetFrame(
        arg1: *mut pyo3::ffi::PyThreadState,
    ) -> *mut pyo3::ffi::PyFrameObject;
    pub unsafe fn PyFrame_GetCode(
        frame: *mut pyo3::ffi::PyFrameObject,
    ) -> *mut pyo3::ffi::PyCodeObject;
    pub unsafe fn PyFrame_GetLineNumber(frame: *mut pyo3::ffi::PyFrameObject) -> i32;
    pub unsafe fn PyFrame_GetBack(
        frame: *mut pyo3::ffi::PyFrameObject,
    ) -> *mut pyo3::ffi::PyFrameObject;
    pub unsafe fn PyFrame_GetLocals(
        frame: *mut pyo3::ffi::PyFrameObject,
    ) -> *mut pyo3::ffi::PyObject;
}

fn get_python_stacks() -> Option<Vec<CallFrame>> {
    let stat = unsafe { pyo3::ffi::PyGILState_Ensure() };
    let frames = Python::with_gil(|py| {
        let global = PyDict::new(py);
        if let Err(err) = py.run(DUMP_STACK, Some(&global), Some(&global)) {
            error!("error extract call stacks {}", err);
            return None;
        }
        match global.get_item("retval") {
            Ok(frames) => {
                if let Some(frames) = frames {
                    frames.extract::<String>().ok()
                } else {
                    error!("error extract python call stacks");
                    None
                }
            }
            Err(err) => {
                error!("error extract python call stacks {}", err);
                None
            }
        }
    });

    let ret = if let Some(frames) = frames {
        serde_json::from_str::<Vec<CallFrame>>(frames.as_str()).ok()
    } else {
        log::error!("Failed to decode Python call stacks");
        None
    };

    unsafe {
        pyo3::ffi::PyGILState_Release(stat);
    }
    ret
}

use cpp_demangle::Symbol;

fn get_native_stacks() -> Option<Vec<CallFrame>> {
    let mut frames = vec![];
    backtrace::trace(|frame| {
        let ip = frame.ip();
        let symbol_address = frame.symbol_address() as usize;
        backtrace::resolve_frame(frame, |symbol| {
            let func = symbol.name().and_then(|name| name.as_str());
            let func = func
                .map(|raw_name| {
                    // 尝试对 C++ 符号名称进行 demangle
                    Symbol::new(raw_name)
                        .ok()
                        .map(|demangled| demangled.to_string())
                        .unwrap_or_else(|| raw_name.to_string())
                })
                .unwrap_or(format!("unknown@{:#x}", symbol_address));

            let file = symbol
                .filename()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or_default();

            frames.push(CallFrame::CFrame {
                ip: format!("{:#x}", ip as usize),
                file,
                func,
                lineno: symbol.lineno().unwrap_or_default() as i64,
            });
        });
        true
    });
    Some(frames)
}

fn merge_python_native_stacks(
    python_stacks: Vec<CallFrame>,
    native_stacks: Vec<CallFrame>,
) -> Vec<CallFrame> {
    let mut merged = vec![];
    let mut python_frame_index = 0;

    enum MergeType {
        Ignore,
        MergeNativeFrame,
        MergePythonFrame,
    }

    fn get_merge_strategy(frame: &CallFrame) -> MergeType {
        lazy_static! {
            static ref WHITELISTED_PREFIXES: HashSet<&'static str> = {
                let mut prefixes = HashSet::new();
                prefixes.insert("time");
                prefixes.insert("sys");
                prefixes.insert("gc");
                prefixes.insert("os");
                prefixes.insert("unicode");
                prefixes.insert("thread");
                prefixes.insert("stringio");
                prefixes.insert("sre");
                // likewise reasoning about lock contention inside python is also useful
                prefixes.insert("PyGilState");
                prefixes.insert("PyThread");
                prefixes.insert("lock");
                prefixes
            };
        }
        let symbol = match frame {
            CallFrame::CFrame {
                ip: _,
                file: _,
                func,
                lineno: _,
            } => func,
            CallFrame::PyFrame {
                file: _,
                func,
                lineno: _,
                locals: _,
            } => func,
        };
        let mut tokens = symbol.split(&['_', '.'][..]).filter(|&x| !x.is_empty());
        match tokens.next() {
            Some("PyEval") => match tokens.next() {
                Some("EvalFrameDefault") => MergeType::MergePythonFrame,
                Some("EvalFrameEx") => MergeType::MergePythonFrame,
                _ => MergeType::Ignore,
            },
            Some(prefix) if WHITELISTED_PREFIXES.contains(prefix) => MergeType::MergeNativeFrame,
            _ => MergeType::MergeNativeFrame,
        }
    }

    for frame in native_stacks {
        log::debug!("Processing native frame: {:?}", frame);
        match get_merge_strategy(&frame) {
            MergeType::Ignore => {}
            MergeType::MergeNativeFrame => merged.push(frame),
            MergeType::MergePythonFrame => {
                #[allow(clippy::never_loop)]
                while python_frame_index < python_stacks.len() {
                    merged.push(python_stacks[python_frame_index].clone());
                    break;
                }
                python_frame_index += 1;
            }
        }
    }
    merged
}

pub fn backtrace_signal_handler() {
    log::debug!("Signal handler: Starting to collect call stacks...");
    let python_stacks = get_python_stacks().unwrap_or_default();
    log::debug!(
        "Signal handler: Collected {} Python call stacks",
        python_stacks.len()
    );
    let native_stacks = get_native_stacks().unwrap_or_default();
    log::debug!(
        "Signal handler: Collected {} native call stacks",
        native_stacks.len()
    );

    let merged_stacks = merge_python_native_stacks(python_stacks, native_stacks);
    log::debug!(
        "Signal handler: Merged call stacks, total {} frames",
        merged_stacks.len()
    );
    if let Ok(guard) = CALLSTACK_SENDER_SLOT.try_lock() {
        if let Some(sender) = guard.as_ref() {
            if let Err(e) = sender.send(merged_stacks) {
                error!("Signal handler: Failed to send callstack data: {}", e);
            }
        } else {
            error!("Signal handler: No active callstack sender found in CALLSTACK_SENDER_SLOT.");
        }
    } else {
        error!("Signal handler: Failed to lock CALLSTACK_SENDER_SLOT mutex.");
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
