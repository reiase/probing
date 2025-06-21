use std::ffi::CStr;

use log::error;
use probing_proto::prelude::CallFrame;
use pyo3::{ffi::c_str, prelude::*, types::PyDict};

const DUMP_STACK_SCRIPT: &str = include_str!("get_stack.py");

pub fn get_python_stacks() -> Option<Vec<CallFrame>> {
    let frames = Python::with_gil(|py| {
        let global = PyDict::new(py);
        if let Err(err) = py.run(DUMP_STACK_SCRIPT, Some(&global), Some(&global)) {
            error!("Failed to execute Python stack dump script: {}", err);
            return None;
        }
        match global.get_item("retval") {
            Ok(Some(frames_str)) => frames_str.extract::<String>().ok(),
            Ok(None) => {
                error!("Python stack dump script did not return 'retval'");
                None
            }
            Err(err) => {
                error!("Failed to get 'retval' from Python stack dump: {}", err);
                None
            }
        }
    });

    frames.and_then(|s| {
        serde_json::from_str::<Vec<CallFrame>>(&s)
            .map_err(|e| {
                error!("Failed to deserialize Python call stacks: {}", e);
                e
            })
            .ok()
    })
}
