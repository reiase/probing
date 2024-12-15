#[macro_use]
extern crate ctor;

use probing_legacy::get_hostname;
use probing_legacy::register_signal_handler;
use probing_legacy::server::report::start_report_worker;
use probing_legacy::sigusr1_handler;
use probing_proto::cli::CtrlSignal;
use probing_python::PythonProbeFactory;
use probing_server::local_server;
use probing_server::remote_server;
use std::env;
use std::sync::Arc;

use env_logger::Env;
use log::debug;
use log::error;
use nix::libc::SIGUSR1;

#[ctor]
fn setup() {
    let pid = std::process::id();
    eprintln!("Initializing libprobing for process {pid} ...",);
    env_logger::init_from_env(Env::new().filter("PROBING_LOG"));

    let argstr = env::var("PROBING_ARGS").unwrap_or("[]".to_string());
    debug!("Setup libprobing with PROBING_ARGS: {argstr}");
    let cmds: Vec<CtrlSignal> = ron::from_str(argstr.as_str()).unwrap();
    debug!("Setup libprobing with commands: {cmds:?}");

    register_signal_handler(SIGUSR1, sigusr1_handler);
    // register_signal_handler(SIGUSR2, dump_stack2);

    // for cmd in cmds {
    //     ctrl_handler(cmd).unwrap();
    // }
    local_server::start(Arc::new(PythonProbeFactory::default()));

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
        remote_server::start(Some(address), Arc::new(PythonProbeFactory::default()));
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
