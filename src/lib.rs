#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod flags;
mod handlers;
mod repl;
mod server;

use clap::Parser;
// use handlers::crash_handler;
use handlers::dump_stack;
use handlers::pause_process;
use handlers::pprof_handler;
use handlers::PPROF_HOLDER;

pub use crate::flags::ProbeFlags;
use crate::handlers::execute_handler;
use repl::PythonRepl;
use server::start_async_server;
use signal_hook::consts::*;
use std::ffi::c_int;
use std::fs;
use std::{env, thread};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook::low_level::register(sig, handler).unwrap() };
}

fn sigusr1_handler() {
    let args = {
        if let Ok(argstr) = env::var("PROBE_ARGS") {
            let split_args = argstr.split(" ");
            ProbeFlags::parse_from(split_args)
        } else {
            ProbeFlags::default()
        }
    };
    eprintln!("in signal USR1: {:?}", args);
    if args.pause {
        pause_process(args.address)
    } else if args.crash {
        // let tmp = args.address.clone();
        // register_signal_handler(SIGABRT, move || crash_handler(tmp.clone()));
    } else if args.background {
        thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(start_async_server::<PythonRepl>(args.address))
                .unwrap();
        });
    } else if args.pprof {
        let _ = PPROF_HOLDER.lock().map(|pp| {});
    } else if let Some(script) = args.execute {
        execute_handler(script)
    }
}

#[cfg(feature = "dll_init")]
#[no_mangle]
#[ctor]
fn init() {
    use clap::Parser;

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
        if let Ok(args) = env::var("PROBE_ARGS") {
            eprintln!("{}: loading libprob with `{}`", _path.display(), args);
        } else {
            eprintln!("{}: loading libprob ", _path.display());
        }
    }
    let args = {
        if let Ok(argstr) = env::var("PROBE_ARGS") {
            let split_args = argstr.split(" ");
            ProbeFlags::parse_from(split_args)
        } else {
            ProbeFlags::default()
        }
    };
    // eprintln!("parsed args: {:?}", args);

    register_signal_handler(SIGUSR1, sigusr1_handler);
    register_signal_handler(SIGUSR2, dump_stack);
    register_signal_handler(SIGPROF, pprof_handler);
    let addr = args.address.clone();
    if args.background {
        thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(start_async_server::<PythonRepl>(args.address))
                .unwrap();
        });
    }
    // if args.crash {
    //     let tmp = addr.clone();
    //     register_signal_handler(SIGABRT, move || crash_handler(tmp.clone()));
    // }
    if args.pprof {
        let _ = PPROF_HOLDER.lock().map(|pp| {});
    }
}
