use std::ffi::CString;

use log::error;
use pyo3::{prelude::*, types::PyDict};

const STACK_THREADS: &str = include_str!("stack_get_threads.py");

pub fn get_python_stacks(tid: i32) -> Option<String> {
    log::debug!("Collecting python backtrace for TID: {tid:?}");

    Python::with_gil(|py| {
        log::debug!("Start calling backtrace for TID: {tid:?}");
        let global = PyDict::new(py);
        global.set_item("tid", tid).unwrap_or_else(|err| {
            error!("Failed to set 'tid' in Python global dict: {err}");
        });
        let script_cstr = CString::new(STACK_THREADS).unwrap_or_default();
        if let Err(err) = py.run(&script_cstr, Some(&global), Some(&global)) {
            error!("Failed to execute Python stack dump script: {err}");
            return None;
        }
        match global.get_item("retval") {
            Ok(Some(frames_str)) => frames_str.extract::<String>().ok(),
            Ok(None) => {
                error!("Python stack dump script did not return 'retval'");
                None
            }
            Err(err) => {
                error!("Failed to get 'retval' from Python stack dump: {err}");
                None
            }
        }
    })
}