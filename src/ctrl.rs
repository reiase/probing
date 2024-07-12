use anyhow::Result;
use nix::libc::SIGABRT;
use probing_common::cli::ProbingCommand;

use crate::{
    handlers::{
        crash_handler, dump_stack, execute_handler, pause_process, pprof_handler, show_plt,
    },
    register_signal_handler,
    repl::PythonRepl,
    server::remote_server,
    service::CALLSTACK,
};

pub fn ctrl_handler(cmd: ProbingCommand) -> Result<()> {
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
            repl.process(cmd.as_str());
        }
        ProbingCommand::Pause { address } => pause_process(address),
        ProbingCommand::Perf => pprof_handler(),
        ProbingCommand::CatchCrash => {
            register_signal_handler(SIGABRT, move || crash_handler(None));
        }
        ProbingCommand::ListenRemote { address } => remote_server::start::<PythonRepl>(address),
        ProbingCommand::Execute { script } => execute_handler(script)?,
        ProbingCommand::ShowPLT => {
            show_plt()?;
        }
    };
    Ok(())
}

pub fn ctrl_handler_string(cmdstr: String) {
    if cmdstr.starts_with('[') {
        let cmds: Vec<ProbingCommand> = ron::from_str(&cmdstr).unwrap();
        for cmd in cmds {
            let _ = ctrl_handler(cmd).map_err(|err| eprintln!("{}", err));
        }
    } else {
        let cmd: ProbingCommand = ron::from_str(&cmdstr).unwrap_or(ProbingCommand::Nil);
        let _ = ctrl_handler(cmd).map_err(|err| eprintln!("{}", err));
    }
}
