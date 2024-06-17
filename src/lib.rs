#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod handlers;
mod repl;
mod server;
mod service;

// use handlers::crash_handler;

use anyhow::Ok;
use handlers::dump_stack;
use handlers::dump_stack2;
use handlers::execute_handler;
use handlers::pause_process;
use handlers::pprof_handler;
use handlers::PPROF_HOLDER;
use probe_common::cli::ProbeCommand;

use crate::service::CALLSTACK;

use anyhow::Result;
use repl::PythonRepl;
use server::start_async_server;
use signal_hook::consts::*;
use std::ffi::c_int;
use std::{env, thread};

fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook::low_level::register(sig, handler).unwrap() };
}

fn probe_command_handler(cmd: ProbeCommand) -> Result<()> {
    match cmd {
        ProbeCommand::Nil => {}
        ProbeCommand::Dump => {
            let ret = dump_stack()?;
            CALLSTACK
                .lock()
                .map(|mut cs| {
                    cs.replace(ret);
                })
                .unwrap();
        }
        ProbeCommand::Pause { address } => pause_process(address),
        ProbeCommand::Pprof => PPROF_HOLDER.setup(1000),
        ProbeCommand::CatchCrash => {
            //     // let tmp = args.address.clone();
            //     // register_signal_handler(SIGABRT, move || crash_handler(tmp.clone()));
        }
        ProbeCommand::ListenRemote { address } => {
            thread::spawn(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(start_async_server::<PythonRepl>(address))
                    .unwrap();
            });
        }
        ProbeCommand::Execute { script } => execute_handler(script)?,
    };
    Ok(())
}

fn sigusr1_handler() {
    let argstr = env::var("PROBE_ARGS").unwrap_or("Nil".to_string());
    let cmd: ProbeCommand = ron::from_str(&argstr).unwrap();

    eprintln!("handling signal USR1 with args: {}", argstr);
    probe_command_handler(cmd).unwrap();
}

#[no_mangle]
#[ctor]
fn init() {
    let argstr = env::var("PROBE_ARGS").unwrap_or("[]".to_string());
    eprintln!("parse args: {}", argstr);
    let probe_commands: Vec<ProbeCommand> = ron::from_str(argstr.as_str()).unwrap();

    register_signal_handler(SIGUSR1, sigusr1_handler);
    register_signal_handler(SIGUSR2, dump_stack2);
    register_signal_handler(SIGPROF, pprof_handler);
    for cmd in probe_commands {
        probe_command_handler(cmd).unwrap();
    }
}
