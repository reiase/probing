use std::{ffi::CStr, sync::Mutex};

use dlopen2::wrapper::{Container, WrapperApi};
use lazy_static::lazy_static;

#[derive(WrapperApi)]
struct PyApi {
    Py_Initialize: fn() -> (),
    Py_GetVersion: fn() -> &'static CStr,
    PyRun_SimpleString: fn(code: &'static CStr) -> i32,
    printf: fn(code: &'static CStr) -> i32,
}
pub struct NativeVM {}

impl Default for NativeVM {
    fn default() -> Self {
        NativeVM {}
    }
}

lazy_static! {
    pub static ref NPYVM: Mutex<Option<NativeVM>> = Mutex::new({
        let prog: Option<Container<PyApi>> = unsafe { Container::load_self() }
            .map_err(|_| eprintln!("probe: no native python detected."))
            .ok();
        if let Some(_) = prog {
            eprintln!("probe: use native python backend.");
            Some(NativeVM {})
        } else {
            None
        }
    });
}
