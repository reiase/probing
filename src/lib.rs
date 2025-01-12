#[macro_use]
extern crate ctor;

use std::env;

use env_logger::Env;
use log::debug;
use log::error;
use nix::libc::SIGUSR1;
use nix::libc::SIGUSR2;

use probing_legacy::ctrl::ctrl_handler;
use probing_legacy::get_hostname;
use probing_legacy::register_signal_handler;
use probing_legacy::sigusr1_handler;
use probing_proto::cli::CtrlSignal;
use probing_python::backtrace_signal_handler;
use probing_python::create_probing_module;

const ENV_PROBING_LOG: &str = "PROBING_LOG";
const ENV_PROBING_ARGS: &str = "PROBING_ARGS";
const ENV_PROBING_PORT: &str = "PROBING_PORT";

const DEFAULT_PORT: u16 = 9700;

#[ctor]
fn setup() {
    let pid = std::process::id();
    eprintln!("Initializing libprobing for process {pid} ...",);
    env_logger::init_from_env(Env::new().filter(ENV_PROBING_LOG));

    let argstr = env::var(ENV_PROBING_ARGS).unwrap_or("[]".to_string());
    debug!("Setup libprobing with PROBING_ARGS: {argstr}");
    let cmds = ron::from_str::<Vec<CtrlSignal>>(argstr.as_str());
    debug!("Setup libprobing with commands: {cmds:?}");

    register_signal_handler(SIGUSR1, sigusr1_handler);
    register_signal_handler(SIGUSR2, backtrace_signal_handler);

    if let Ok(cmds) = cmds {
        for cmd in cmds {
            ctrl_handler(cmd).unwrap();
        }
    }
    probing_server::start_local();

    if let Ok(port) = std::env::var(ENV_PROBING_PORT) {
        let local_rank = std::env::var("LOCAL_RANK")
            .unwrap_or("0".to_string())
            .parse()
            .unwrap_or(0);
        let hostname = if std::env::var("RANK").unwrap_or("0".to_string()) == "0" {
            "0.0.0.0".to_string()
        } else {
            get_hostname().unwrap_or("localhost".to_string())
        };

        let address = format!(
            "{}:{}",
            hostname,
            port.parse().unwrap_or(DEFAULT_PORT) + local_rank
        );
        println!(
            "Starting remote server for process {} at {}",
            std::process::id(),
            address
        );
        probing_server::start_remote(Some(address));
        probing_server::start_report_worker();
    }
    let _ = create_probing_module();
}

#[dtor]
fn cleanup() {
    if let Err(e) = probing_server::cleanup() {
        error!("Failed to cleanup unix socket: {}", e);
    }
}
