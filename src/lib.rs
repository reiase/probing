#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod prof;
mod repl;
mod server;

use prof::PPROF;
use repl::PythonRepl;
use repl::RustPythonRepl;
use repl::PYVM;
use std::{env, io::Error, thread};

use std::fs;

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

#[ctor]
fn init() {
    if let Ok(_path) = fs::read_link("/proc/self/exe") {
        let path_str = _path.to_string_lossy();
        if path_str.ends_with("/probe")
            || path_str.ends_with("/bash")
            || path_str.ends_with("/sh")
            || path_str.ends_with("/zsh")
            || path_str.ends_with("/dash")
        {
            return;
        }
        eprintln!("{}: loading libprob", _path.display());
    }
    let _ = enable_debug_server(
        env::var("PROBE_ADDR").ok(),
        env::var("PROBE_BG").map(|_| true).unwrap_or(false),
        env::var("PROBE_PPROF").map(|_| true).unwrap_or(false),
    );
}
