#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod prof;
mod repl;
mod server;

use lazy_static::lazy_static;
use prof::PPROF;
use repl::PythonRepl;
use repl::RustPythonRepl;
use repl::PYVM;
use std::{env, io::Error, thread};

use std::fs;
use std::sync::Mutex;

use pyo3::prelude::*;

use server::start_async_server;
use server::start_debug_server;

pub fn debug_callback(addr: Option<String>) {
    let mut repl = RustPythonRepl::default();
    start_debug_server(addr, &mut repl);
}

pub fn enable_debug_server(
    addr: Option<String>,
    background: bool,
    pprof: bool,
) -> Result<(), Error> {
    unsafe {
        let tmp = addr.clone();
        signal_hook::low_level::register(signal_hook::consts::SIGUSR1, move || {
            debug_callback(tmp.clone())
        })?;
        let tmp = addr.clone();
        signal_hook::low_level::register(signal_hook::consts::SIGABRT, move || {
            debug_callback(tmp.clone())
        })?;
    }
    if background {
        thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(start_async_server::<PythonRepl>(addr))
                .unwrap();
        });
    }
    if pprof {
        let _ = PYVM.lock().map(|vm| {});
        let _ = PPROF.lock().map(|pp| {});
    }
    Ok(())
}

lazy_static! {
    static ref NPY: Mutex<i32> = Mutex::new({
        Python::with_gil(|py| {
            let _ = py
                .eval_bound("print('------')", None, None)
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                })
                .unwrap();
        });
        1
    });
}

#[ctor]
fn init() {
    if let Ok(_path) = fs::read_link("/proc/self/exe") {
        if _path.to_string_lossy().ends_with("/probe") {
            return;
        }
        eprintln!("{}: loading libprob", _path.display());
    }
    let _ = enable_debug_server(
        env::var("PROBE_ADDR").ok(),
        env::var("PROBE_BG").map(|_| true).unwrap_or(false),
        env::var("PROBE_PPROF").map(|_| true).unwrap_or(false),
    );
    // let _ = PYVM.lock().map(|pyvm| {
    //     pyvm.interp.enter(|vm| {
    //         let scope = vm.new_scope_with_builtins();
    //         let _ = vm.run_block_expr(
    //             scope,
    //             "import sys;print('libprob has been loaded', file=sys.stderr)",
    //         );
    //     })
    // });

    // #[derive(WrapperApi)]
    // struct PyApi {
    //     Py_Initialize: fn() -> (),
    //     Py_GetVersion: fn() -> &'static CStr,
    //     PyRun_SimpleString: fn(code: &'static CStr) -> i32,
    //     printf: fn(code: &'static CStr) -> i32,
    // }

    // let prog: Option<Container<PyApi>> = unsafe { Container::load_self() }
    //     .map_err(|err| eprintln!("!!!!{}", err))
    //     .ok();
    // if let Some(prog) = prog {
    //     (prog.printf)(const_cstr!("print('====')\n").as_cstr());
    // }
}
