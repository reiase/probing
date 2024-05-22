#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod handlers;
mod repl;
mod server;

use handlers::crash_handler;
use handlers::dump_stack;
use handlers::pause_process;
use handlers::PPROF_HOLDER;

use repl::PythonRepl;
use repl::PYVM;
use std::{env, io::Error, thread};

use std::fs;

use server::start_async_server;

pub fn enable_probe_server(
    addr: Option<String>,
    background: bool,
    pprof: bool,
) -> Result<(), Error> {
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGUSR2, move || dump_stack())?;
        let tmp = addr.clone();
        signal_hook::low_level::register(signal_hook::consts::SIGUSR1, move || {
            pause_process(tmp.clone())
        })?;
        let tmp = addr.clone();
        signal_hook::low_level::register(signal_hook::consts::SIGABRT, move || {
            crash_handler(tmp.clone())
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
        let _ = PPROF_HOLDER.lock().map(|pp| {});
    }
    Ok(())
}

#[cfg(feature = "dll_init")]
#[no_mangle]
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
    let _ = enable_probe_server(
        env::var("PROBE_ADDR").ok(),
        env::var("PROBE_BG").map(|_| true).unwrap_or(false),
        env::var("PROBE_PPROF").map(|_| true).unwrap_or(false),
    );
}
