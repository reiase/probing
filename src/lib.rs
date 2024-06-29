#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod handlers;
mod repl;
mod server;
mod service;

use std::ffi::c_int;
use std::str::FromStr as _;
use std::{env, thread};

use anyhow::Result;
use env_logger::Env;
use log::debug;
use log::info;
use pyo3::prelude::*;
use signal_hook::consts::*;

// use handlers::crash_handler;

use handlers::dump_stack;
use handlers::dump_stack2;
use handlers::execute_handler;
use handlers::pause_process;
use handlers::pprof_handler;

use probe_common::cli::ProbeCommand;
use repl::PythonRepl;
use server::start_async_server;
use service::CALLSTACK;

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
        ProbeCommand::Perf => pprof_handler(),
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

#[ctor]
fn setup() {
    env_logger::init_from_env(Env::new().filter("PROBE_LOG"));
    info!("Initializing libprobe ...");

    let argstr = env::var("PROBE_ARGS").unwrap_or("[]".to_string());
    debug!("Setup libprobe with PROBE_ARGS: {argstr}");
    let probe_commands: Vec<ProbeCommand> = ron::from_str(argstr.as_str()).unwrap();
    debug!("Setup libprobe with commands: {probe_commands:?}");

    register_signal_handler(SIGUSR1, sigusr1_handler);
    register_signal_handler(SIGUSR2, dump_stack2);

    for cmd in probe_commands {
        probe_command_handler(cmd).unwrap();
    }
}

#[pyfunction]
#[pyo3(signature = (address=None, background=true, pprof=false, log_level=None))]
fn init(address: Option<String>, background: bool, pprof: bool, log_level: Option<String>) {
    if let Some(log_level) = log_level {
        log::set_max_level(
            log::LevelFilter::from_str(log_level.as_str()).unwrap_or(log::LevelFilter::Info),
        );
    }

    let mut probe_commands = vec![];
    if background {
        probe_commands.push(ProbeCommand::ListenRemote { address })
    }
    if pprof {
        probe_commands.push(ProbeCommand::Perf)
    }

    debug!("Setup libprobe with commands: {probe_commands:?}");
    for cmd in probe_commands {
        probe_command_handler(cmd).unwrap();
    }
}

#[pymodule]
fn probe(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    Ok(())
}
