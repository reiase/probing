#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod handlers;
mod repl;
mod server;
mod service;

// use handlers::crash_handler;

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

use pyo3::prelude::*;

fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook::low_level::register(sig, handler).unwrap() };
}

pub fn probe_command_handler(cmd: ProbeCommand) -> Result<()> {
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
        ProbeCommand::Perf => PPROF_HOLDER.setup(1000),
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
    if argstr.starts_with('[') {
        let cmds: Vec<ProbeCommand> = ron::from_str(&argstr).unwrap();
        for cmd in cmds {
            probe_command_handler(cmd).unwrap();
        }
    } else {
        let cmd: ProbeCommand = ron::from_str(&argstr).unwrap();
        probe_command_handler(cmd).unwrap();
    }
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

#[pyfunction]
#[pyo3(signature = (address=None, background=true, pprof=false))]
fn initialize(address: Option<String>, background: bool, pprof: bool) {
    let mut probe_commands = vec![];
    if background {
        probe_commands.push(ProbeCommand::ListenRemote{address})
    }
    if pprof {
        probe_commands.push(ProbeCommand::Perf)
    }
    for cmd in probe_commands {
        probe_command_handler(cmd).unwrap();
    }
}

#[pymodule]
fn probe(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(initialize, m)?)?;
    Ok(())
}