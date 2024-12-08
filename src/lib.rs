#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

use anyhow::Result;
use std::{env, ffi::c_int, str::FromStr as _};

use ctrl::{ctrl_handler, ctrl_handler_string};
use env_logger::Env;
use log::debug;
use log::error;
use nix::libc;
use nix::libc::SIGUSR1;
use nix::libc::SIGUSR2;
use pyo3::prelude::*;
use server::local_server;

mod core;
mod ctrl;
mod handlers;
mod hooks;
mod repl;
mod server;
mod service;

pub mod plugins;

use handlers::dump_stack2;
use probing_proto::cli::CtrlSignal;
use probing_proto::cli::Features;
use repl::PythonRepl;
use server::remote_server;
use server::report::start_report_worker;

fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook_registry::register_unchecked(sig, move |_: &_| handler()).unwrap() };
}

fn sigusr1_handler() {
    let cmdstr = env::var("PROBING_ARGS").unwrap_or("Nil".to_string());
    ctrl_handler_string(cmdstr);
}

pub fn get_hostname() -> Result<String> {
    let limit = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) };
    let size = libc::c_long::max(limit, 256) as usize;

    // Reserve additional space for terminating nul byte.
    let mut buffer = vec![0u8; size + 1];

    #[allow(trivial_casts)]
    let result = unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut libc::c_char, size) };

    if result != 0 {
        return Err(anyhow::anyhow!("gethostname failed"));
    }

    let hostname = std::ffi::CStr::from_bytes_until_nul(buffer.as_slice())?;

    Ok(hostname.to_str()?.to_string())
}

#[ctor]
fn setup() {
    eprintln!(
        "Initializing libprobing for process {} ...",
        std::process::id()
    );
    env_logger::init_from_env(Env::new().filter("PROBING_LOG"));

    let argstr = env::var("PROBING_ARGS").unwrap_or("[]".to_string());
    debug!("Setup libprobing with PROBING_ARGS: {argstr}");
    let cmds: Vec<CtrlSignal> = ron::from_str(argstr.as_str()).unwrap();
    debug!("Setup libprobing with commands: {cmds:?}");

    register_signal_handler(SIGUSR1, sigusr1_handler);
    register_signal_handler(SIGUSR2, dump_stack2);

    for cmd in cmds {
        ctrl_handler(cmd).unwrap();
    }
    local_server::start::<PythonRepl>();

    if let Ok(port) = std::env::var("PROBING_PORT") {
        let local_rank = std::env::var("LOCAL_RANK")
            .unwrap_or("0".to_string())
            .parse()
            .unwrap_or(0);
        let hostname = if std::env::var("RANK").unwrap_or("0".to_string()) == "0" {
            "0.0.0.0".to_string()
        } else {
            get_hostname().unwrap_or("localhost".to_string())
        };

        let address = format!("{}:{}", hostname, port.parse().unwrap_or(9700) + local_rank);
        println!(
            "Starting remote server for process {} at {}",
            std::process::id(),
            address
        );
        remote_server::start::<PythonRepl>(Some(address));
        start_report_worker();
    }
}

#[dtor]
fn cleanup() {
    if let Err(e) = local_server::stop() {
        error!("Error cleanup unix socket for {}", std::process::id());
        error!("{}", e);
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

    let mut cmds = vec![];
    if background {
        cmds.push(CtrlSignal::Enable(Features::Remote { address }))
    }
    if pprof {
        cmds.push(CtrlSignal::Enable(Features::Pprof))
    }

    debug!("Setup libprobing with commands: {cmds:?}");
    for cmd in cmds {
        ctrl_handler(cmd).unwrap();
    }
    // local_server::start::<PythonRepl>();
}

#[pymodule]
fn probing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    Ok(())
}
