#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

use anyhow::Result;
use std::{env, ffi::c_int, str::FromStr as _};

use ctrl::{ctrl_handler, ctrl_handler_string};
use env_logger::Env;
use log::debug;
use log::error;
use pyo3::prelude::*;
use server::local_server;
use signal_hook::consts::*;

mod ctrl;
mod handlers;
mod repl;
mod server;
mod service;
mod trace;

use handlers::dump_stack2;
use probing_ppp::cli::CtrlSignal;
use probing_ppp::cli::Features;
use repl::PythonRepl;

use rust_embed::Embed;
#[derive(Embed)]
#[folder = "pys/"]
struct PythonSourceCode;

fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook::low_level::register(sig, handler).unwrap() };
}

fn sigusr1_handler() {
    let cmdstr = env::var("PROBING_ARGS").unwrap_or("Nil".to_string());
    ctrl_handler_string(cmdstr);
}

#[ctor]
fn setup() {
    eprintln!("Initializing libprobing ...");
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
    if let Err(err) = setup_module() {
        error!("Error setting up module: {}", err);
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
    local_server::start::<PythonRepl>();
}

#[pymodule]
fn probing(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init, m)?)?;
    Ok(())
}

fn setup_module() -> Result<()> {
    Python::with_gil(|py| {
        let modules = py.import_bound("sys")?.getattr("modules")?;
        let pi = PyModule::new_bound(py, "probing")?;
        probing(&pi)?;
        modules.set_item("pi", &pi)?;
        modules.set_item("probing", &pi)?;

        let init_code = PythonSourceCode::get("init.py").unwrap();
        let init_code = String::from_utf8(init_code.data.to_vec())?;
        py.run_bound(init_code.as_str(), None, None)?;
        Ok(())
    })
}
