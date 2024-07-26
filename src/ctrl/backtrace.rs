use std::env;

use anyhow::Result;

use nix::{
    sys::signal,
    unistd::{sleep, Pid},
};
use probing_ppp::cli::{BackTraceCommand, CtrlSignal};
use pyo3::Python;

use crate::{
    handlers::{cc_backtrace, py_backtrace},
    repl::PythonRepl,
    server::start_debug_server,
    service::CALLSTACK,
};

pub fn handle(bt: BackTraceCommand) -> Result<String> {
    match bt {
        BackTraceCommand::Show { cc, python, tid } => {
            CALLSTACK
                .lock()
                .map(|mut cs| {
                    *cs = None;
                })
                .unwrap();
            let mut pid = std::process::id();
            if let Some(tid) = tid {
                pid = tid as u32;
            }
            let cmd = CtrlSignal::Backtrace(BackTraceCommand::Trigger { cc, python });
            let cmd = ron::to_string(&cmd).unwrap_or("[]".to_string());
            env::set_var("PROBING_ARGS", cmd);
            signal::kill(Pid::from_raw(pid as i32), signal::SIGUSR1).unwrap();
            sleep(1);
            Ok(CALLSTACK
                .lock()
                .map(|cs| cs.clone().unwrap_or("no call stack".to_string()))
                .unwrap_or("no call stack".to_string()))
        }

        BackTraceCommand::Pause {
            address,
            tid,
            signal,
        } => {
            if signal {
                let mut repl = PythonRepl::default();
                start_debug_server(address, &mut repl);
            } else {
                let tid = tid.unwrap_or(std::process::id());
                let cmd = CtrlSignal::Backtrace(BackTraceCommand::Pause {
                    address: address.clone(),
                    tid: None,
                    signal: true,
                });
                env::set_var(
                    "PROBING_ARGS",
                    ron::to_string(&cmd).unwrap_or("[]".to_string()),
                );
                signal::kill(Pid::from_raw(tid as i32), signal::SIGUSR1).unwrap();
            }
            Ok(Default::default())
        }

        BackTraceCommand::Trigger { cc, python } => Python::with_gil(|_| {
            let mut ret = String::new();
            if python {
                ret.push_str(py_backtrace().as_str());
            }
            if cc {
                ret.push_str(cc_backtrace().as_str());
            }
            CALLSTACK
                .lock()
                .map(|mut cs| {
                    cs.replace(ret);
                })
                .unwrap();
            Ok(Default::default())
        }),
    }
}
