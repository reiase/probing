// use std::sync::Arc;

// use log::{error, info};
use anyhow::Result;
use nix::libc::{SIGABRT, SIGBUS, SIGFPE, SIGSEGV};
use probing_proto::cli::Features;
// use probing_python::PythonProbeFactory;

use crate::{
    handlers::pprof_handler,
    register_signal_handler,
    repl::PythonRepl,
    // server::{remote_server, start_debug_server},
};

pub fn handle(feature: Features) -> Result<String> {
    log::debug!("enable feature: {:?}", feature);
    match feature {
        Features::Pprof => {
            pprof_handler();
            Ok(Default::default())
        }
        Features::Dap { address } => {
            let mut repl = PythonRepl::default();
            let cmd = if let Some(addr) = address {
                if addr.contains(':') {
                    let addr = addr.split(':').collect::<Vec<&str>>();
                    let host = addr[0];
                    let port = addr[1];
                    format!("debug(\"{}\", {})", host, port)
                } else {
                    "debug()".to_string()
                }
            } else {
                "debug()".to_string()
            };
            repl.process(cmd.as_str());
            Ok(Default::default())
        }
        Features::Remote { address } => {
            probing_server::start_remote(address);
            // probing_server::start_remote(address, Arc::new(PythonProbeFactory::default()));

            // remote_server::start::<PythonRepl>(address);
            Ok(Default::default())
        }
        Features::CatchCrash { address } => {
            let addr = address.clone();
            register_signal_handler(SIGABRT, move || crash_handler(addr.clone()));

            let addr = address.clone();
            register_signal_handler(SIGBUS, move || crash_handler(addr.clone()));

            let addr = address.clone();
            register_signal_handler(SIGSEGV, move || crash_handler(addr.clone()));

            let addr = address.clone();
            register_signal_handler(SIGFPE, move || crash_handler(addr.clone()));

            Ok(Default::default())
        }
    }
}

fn crash_handler(addr: Option<String>) {
    let repl = PythonRepl::default();
    // start_debug_server(addr, &mut repl);
}
