use pyo3::{
    types::{PyAnyMethods, PyDict},
    Bound, Py, PyAny, Python,
};
use std::env;
use std::fs;

use crate::repl::repl::PythonConsole;

pub const CODE: &str = include_str!("debug_console.py");

fn get_repl_code() -> String {
    if let Ok(code_path) = env::var("PROBE_REPL_CODE") {
        if let Ok(content) = fs::read_to_string(code_path.clone()) {
            return content;
        }
    }
    return CODE.to_string();
}

pub struct NativePythonConsole {
    console: Py<PyAny>,
}

impl Default for NativePythonConsole {
    #[inline(never)]
    fn default() -> Self {
        Self {
            console: Python::with_gil(|py| {
                let global = PyDict::new_bound(py);
                let code = get_repl_code();
                let _ = py.run_bound(code.as_str(), Some(&global), Some(&global));
                let ret: Bound<'_, PyAny> = global
                    .get_item("debug_console")
                    .map_err(|err| {
                        eprintln!("error initializing console: {}", err);
                    })
                    .unwrap();
                ret.unbind()
            }),
        }
    }
}

impl PythonConsole for NativePythonConsole {
    fn try_execute(&mut self, cmd: String) -> Option<String> {
        Python::with_gil(|py| match self.console.call_method1(py, "push", (cmd,)) {
            Ok(obj) => {
                if obj.is_none(py) {
                    None
                } else {
                    Some(obj.to_string())
                }
            }
            Err(err) => Some(err.to_string()),
        })
    }
}
