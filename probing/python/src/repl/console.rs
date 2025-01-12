use pyo3::{
    types::{PyAnyMethods, PyDict},
    Bound, Py, PyAny, Python,
};

use crate::repl::python_repl::PythonConsole;

#[cfg(not(debug_assertions))]
use include_dir::{include_dir, Dir};

#[cfg(not(debug_assertions))]
static ASSET: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/repl/");

#[cfg(debug_assertions)]
fn get_repl_code() -> String {
    std::fs::read_to_string("src/repl/debug_console.py").unwrap_or_default()
}

#[cfg(not(debug_assertions))]
fn get_repl_code() -> String {
    let code = ASSET.get_file("debug_console.py").unwrap_or_default();    
    code.contents_utf8().unwrap_or_default().to_string()
}

pub struct NativePythonConsole {
    console: Py<PyAny>,
}

impl Default for NativePythonConsole {
    #[inline(never)]
    fn default() -> Self {
        Self {
            console: Python::with_gil(|py| {
                let global = PyDict::new(py);
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
