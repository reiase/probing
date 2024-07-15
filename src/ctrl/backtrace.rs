use std::env;

use anyhow::Result;

use nix::{
    sys::signal,
    unistd::{sleep, Pid},
};
use probing_common::cli::{BackTraceCommand, CtrlSignal};
use pyo3::Python;

use crate::{
    handlers::{cc_backtrace, py_backtrace},
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
