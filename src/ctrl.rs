use anyhow::Result;
use nix::libc::SIGABRT;
use probing_common::cli::CtrlSignal;

use crate::{
    handlers::{
        crash_handler, dump_stack, execute_handler, pause_process, pprof_handler, show_plt,
    },
    register_signal_handler,
    repl::PythonRepl,
    server::remote_server,
    service::CALLSTACK,
};

pub fn ctrl_handler(cmd: CtrlSignal) -> Result<()> {
    match cmd {
        CtrlSignal::Nil => {}
        CtrlSignal::Dump => {
            let ret = dump_stack()?;
            CALLSTACK
                .lock()
                .map(|mut cs| {
                    cs.replace(ret);
                })
                .unwrap();
        }
        CtrlSignal::Dap { address } => {
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
        }
        CtrlSignal::Pause { address } => pause_process(address),
        CtrlSignal::Perf => pprof_handler(),
        CtrlSignal::CatchCrash => {
            register_signal_handler(SIGABRT, move || crash_handler(None));
        }
        CtrlSignal::ListenRemote { address } => remote_server::start::<PythonRepl>(address),
        CtrlSignal::Execute { script } => execute_handler(script)?,
        CtrlSignal::ShowPLT => {
            show_plt()?;
        }

        _ => (),
    };
    Ok(())
}

pub fn ctrl_handler_string(cmdstr: String) {
    if cmdstr.starts_with('[') {
        let cmds: Vec<CtrlSignal> = ron::from_str(&cmdstr).unwrap();
        for cmd in cmds {
            let _ = ctrl_handler(cmd).map_err(|err| eprintln!("{}", err));
        }
    } else {
        let cmd: CtrlSignal = ron::from_str(&cmdstr).unwrap_or(CtrlSignal::Nil);
        let _ = ctrl_handler(cmd).map_err(|err| eprintln!("{}", err));
    }
}
