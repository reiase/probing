use pyo3::ffi::c_str;
use pyo3::{
    types::{PyAnyMethods, PyDict},
    Bound, Py, PyAny, Python,
};

use crate::repl::python_repl::PythonConsole;

pub struct NativePythonConsole {
    console: Py<PyAny>,
}

impl Default for NativePythonConsole {
    #[inline(never)]
    fn default() -> Self {
        Self {
            console: Python::with_gil(|py| {
                let global = PyDict::new(py);
                let code = c_str!("from probing.repl import debug_console");
                let _ = py.run(code, Some(&global), Some(&global));
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

#[cfg(test)]
mod test {
    use crate::repl::python_repl::PythonConsole;

    #[test]
    fn test_python_console() {
        let mut console = super::NativePythonConsole::default();
        let ret = console.try_execute("1+1".to_string());
        assert_eq!(ret, Some("2\n".to_string()));
    }
}
