use pyo3::{
    types::{PyAnyMethods, PyDict},
    Bound, Py, PyAny, Python,
};

use rust_embed::Embed;

use crate::repl::python_repl::PythonConsole;

#[derive(Embed)]
#[folder = "src/repl/"]
struct Asset;

fn get_repl_code() -> String {
    let code = Asset::get("debug_console.py").unwrap();
    String::from_utf8(code.data.to_vec()).unwrap()
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
