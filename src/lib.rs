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

use handlers::dump_stack2;
use handlers::execute_handler;
use handlers::pause_process;
use handlers::pprof_handler;
use handlers::{dump_stack, show_plt};

use probing_common::cli::ProbingCommand;
use repl::PythonRepl;
use server::start_local_server;
use server::start_remote_server;
use service::CALLSTACK;

fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook::low_level::register(sig, handler).unwrap() };
}

pub fn probing_command_handler(cmd: ProbingCommand) -> Result<()> {
    println!("=={:?}", cmd);
    match cmd {
        ProbingCommand::Nil => {}
        ProbingCommand::Dump => {
            let ret = dump_stack()?;
            CALLSTACK
                .lock()
                .map(|mut cs| {
                    cs.replace(ret);
                })
                .unwrap();
        }
        ProbingCommand::Dap { address } => {
            let mut repl = PythonRepl::default();
            let cmd = if let Some(addr) = address {
                if addr.contains(':') {
                    let addr = addr.split(':').collect::<Vec<&str>>();
                    let host = addr[0];
                    let port = addr[1];
                    format!("debug(\"{}\", {})", host, port)
                } else {
                    format!("debug()")
                }
            } else {
                format!("debug()")
            };
            println!("==== {}", cmd);
            repl.process(cmd.as_str());
        }
        ProbingCommand::Pause { address } => pause_process(address),
        ProbingCommand::Perf => pprof_handler(),
        ProbingCommand::CatchCrash => {
            //     // let tmp = args.address.clone();
            //     // register_signal_handler(SIGABRT, move || crash_handler(tmp.clone()));
        }
        ProbingCommand::ListenRemote { address } => {
            thread::spawn(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(start_remote_server::<PythonRepl>(address))
                    .unwrap();
            });
        }
        ProbingCommand::Execute { script } => execute_handler(script)?,
        ProbingCommand::ShowPLT => {
            show_plt()?;
        }
    };
    Ok(())
}

fn sigusr1_handler() {
    let argstr = env::var("PROBING_ARGS").unwrap_or("Nil".to_string());
    if argstr.starts_with('[') {
        let cmds: Vec<ProbingCommand> = ron::from_str(&argstr).unwrap();
        for cmd in cmds {
            let _ = probing_command_handler(cmd).map_err(|err| eprintln!("{}", err));
        }
    } else {
        let cmd: ProbingCommand = ron::from_str(&argstr).unwrap_or(ProbingCommand::Nil);
        let _ = probing_command_handler(cmd).map_err(|err| eprintln!("{}", err));
    }
}

#[ctor]
fn setup() {
    eprintln!("Initializing libprobing ...");
    env_logger::init_from_env(Env::new().filter("PROBING_LOG"));

    let argstr = env::var("PROBING_ARGS").unwrap_or("[]".to_string());
    debug!("Setup libprobing with PROBING_ARGS: {argstr}");
    let cmds: Vec<ProbingCommand> = ron::from_str(argstr.as_str()).unwrap();
    debug!("Setup libprobing with commands: {cmds:?}");

    register_signal_handler(SIGUSR1, sigusr1_handler);
    register_signal_handler(SIGUSR2, dump_stack2);

    for cmd in cmds {
        probing_command_handler(cmd).unwrap();
    }
    thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(start_local_server::<PythonRepl>())
            .unwrap();
    });
}

#[pyfunction]
#[pyo3(signature = (address=None, background=true, pprof=false, log_level=None))]
fn init(address: Option<String>, background: bool, pprof: bool, log_level: Option<String>) {
    if let Some(log_level) = log_level {
        log::set_max_level(
            log::LevelFilter::from_str(log_level.as_str()).unwrap_or(log::LevelFilter::Info),
        );
    }

    let mut cmds = vec![];
    if background {
        cmds.push(ProbingCommand::ListenRemote { address })
    }
    if pprof {
        cmds.push(ProbingCommand::Perf)
    }

    debug!("Setup libprobing with commands: {cmds:?}");
    for cmd in cmds {
        probing_command_handler(cmd).unwrap();
    }
    thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(start_local_server::<PythonRepl>())
            .unwrap();
    });
}

#[pymodule]
fn probing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    Ok(())
}
