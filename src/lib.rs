#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod handlers;
mod repl;
mod server;

use handlers::crash_handler;
use handlers::dump_stack;
use handlers::pause_process;
use handlers::pprof_handler;
use handlers::PPROF_HOLDER;

use repl::PythonRepl;
use repl::PYVM;
use signal_hook::SigId;
use std::ffi::c_int;
use std::sync::Mutex;
use std::{env, io::Error, thread};

use signal_hook::consts::*;

use std::collections::HashMap;
use std::fs;

use lazy_static::lazy_static;

use server::start_async_server;

lazy_static! {
    pub static ref SIGMAP: Mutex<HashMap<c_int, SigId>> = Mutex::new(HashMap::default());
}

fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    let sigid = unsafe { signal_hook::low_level::register(sig, handler).unwrap() };
    let _ = SIGMAP.lock().map(|mut m| m.insert(sig, sigid));
}

pub fn enable_probe_server(
    addr: Option<String>,
    background: bool,
    pprof: bool,
) -> Result<(), Error> {
    {
        register_signal_handler(SIGUSR2, dump_stack);
        let tmp = addr.clone();
        register_signal_handler(SIGUSR1, move || pause_process(tmp.clone()));
        register_signal_handler(SIGPROF, pprof_handler);
        let tmp = addr.clone();
        register_signal_handler(SIGABRT, move || crash_handler(tmp.clone()));
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
        eprintln!("{:?}", env::var("PROBE_ENABLED"));
    }
    let _ = enable_probe_server(
        env::var("PROBE_ADDR").ok(),
        env::var("PROBE_BG").map(|_| true).unwrap_or(false),
        env::var("PROBE_PPROF").map(|_| true).unwrap_or(false),
    );
}
