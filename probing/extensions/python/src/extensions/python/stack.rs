use std::ffi::CString;

use log::error;
use probing_proto::prelude::CallFrame;
use pyo3::{prelude::*, types::PyDict};

const STACK_CURRENT: &str = include_str!("stack_get_current.py");

pub fn get_python_stacks() -> Option<Vec<CallFrame>> {
    let frames = Python::with_gil(|py| {
        let global = PyDict::new(py);
        let script_cstr = CString::new(STACK_CURRENT).unwrap();
        if let Err(err) = py.run(&script_cstr, Some(&global), Some(&global)) {
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
