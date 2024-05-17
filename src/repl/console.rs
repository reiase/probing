use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use pyo3::{
    types::{PyAnyMethods, PyDict},
    Bound, Py, PyAny, Python,
};
use rustpython::vm::{AsObject, PyObjectRef};

use crate::repl::repl::PythonConsole;

use crate::repl::npy_repl::NPYVM;
use crate::repl::rpy_repl::PYVM;

use super::CODE;

pub struct RustPythonConsole {
    console: Option<PyObjectRef>,
}

impl Default for RustPythonConsole {
    fn default() -> Self {
        // let has_native_python = NPYVM
        //     .lock()
        //     .map(|nvm| if nvm.is_none() { false } else { true })
        //     .unwrap();

        let rpy = PYVM
            .lock()
            .map(|pyvm| {
                pyvm.interp
                    .enter(|vm| pyvm.scope.get_item("debug_console", vm).unwrap())
            })
            .unwrap();
        Self { console: Some(rpy) }
    }
}

impl PythonConsole for RustPythonConsole {
    fn try_execute(&mut self, cmd: String) -> Option<String> {
        let ret = self.console.as_ref().map(|console| {
            PYVM.lock()
                .map(|pyvm| {
                    pyvm.interp.enter(|vm| {
                        let args = cmd.to_string();
                        let func = console.as_ref().get_attr("push", vm).unwrap();
                        let ret = func.call((args,), vm);
                        match ret {
                            Ok(obj) => {
                                if vm.is_none(&obj) {
                                    None
                                } else {
                                    Some(obj.str(vm).unwrap().to_string())
                                }
                            }
                            Err(err) => Some(err.as_object().str(vm).unwrap().to_string()),
                        }
                    })
                })
                .unwrap()
        });
        ret?
    }
}

pub struct NativePythonConsole {
    console: Option<Py<PyAny>>,
}

impl Default for NativePythonConsole {
    #[inline(never)]
    fn default() -> Self {
        let console = Python::with_gil(|py| {
            let global = PyDict::new_bound(py);
            let _ = py.run_bound(CODE, Some(&global), Some(&global));
            let ret: Bound<'_, PyAny> = global.get_item("debug_console").unwrap();
            ret.unbind()
        });
        Self {
            console: Some(console),
        }
    }
}

impl PythonConsole for NativePythonConsole {
    fn try_execute(&mut self, cmd: String) -> Option<String> {
        let ret = self.console.as_ref().map(|console| {
            Python::with_gil(|py| {
                // let func = console.getattr(py, "push").unwrap();
                // let ret = func.call_method1(py, name, args)
                let ret = console.call_method1(py, "push", (cmd,));
                match ret {
                    Ok(obj) => {
                        if obj.is_none(py) {
                            None
                        } else {
                            Some(obj.to_string())
                        }
                    }
                    Err(err) => Some(err.to_string()),
                }
            })
        });
        ret?
    }
}

lazy_static! {
    pub static ref SharedNativeConsole: Arc<Mutex<dyn PythonConsole + Send>> =
        Arc::new(Mutex::new(NativePythonConsole::default()));
}
