use anyhow::Result;
use nix::libc::SIGABRT;
use probing_ppp::cli::Features;

use crate::{
    handlers::pprof_handler,
    register_signal_handler,
    repl::PythonRepl,
    server::{remote_server, start_debug_server},
};

pub fn handle(feature: Features) -> Result<String> {
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
            remote_server::start::<PythonRepl>(address);
            Ok(Default::default())
        }
        Features::CatchCrash { address } => {
            register_signal_handler(SIGABRT, move || crash_handler(address.clone()));
            Ok(Default::default())
        }
    }
}

fn crash_handler(addr: Option<String>) {
    let mut repl = PythonRepl::default();
    start_debug_server(addr, &mut repl);
}
