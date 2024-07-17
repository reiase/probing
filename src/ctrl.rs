use anyhow::Result;
use nix::libc::SIGABRT;
use probing_common::cli::{BackTraceCommand, CtrlSignal, Features, ShowCommand};

use crate::{
    handlers::{
        crash_handler, dump_stack, execute_handler, pause_process, pprof_handler, show_plt,
    },
    register_signal_handler,
    repl::PythonRepl,
    server::remote_server,
    service::CALLSTACK,
};

use probing_common::cli::CtrlSignal::Enable;
use probing_common::cli::CtrlSignal::Show;

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

        cmd => {
            handle_ctrl(cmd)?;
        }
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

mod backtrace;
mod disable;
mod enable;
mod eval;
mod show;

#[derive(Default)]
pub struct StringBuilder {
    buf: String,
}

impl ToString for StringBuilder {
    fn to_string(&self) -> String {
        self.buf.clone()
    }
}

pub trait StringBuilderAppend {
    fn append_line(&self, builder: &mut StringBuilder);
}

impl StringBuilderAppend for String {
    fn append_line(&self, builder: &mut StringBuilder) {
        builder.buf.push_str(self.as_str());
        builder.buf.push_str("\n");
    }
}

pub fn handle_ctrl(ctrl: CtrlSignal) -> Result<String> {
    match ctrl {
        CtrlSignal::Nil => Ok(Default::default()),
        CtrlSignal::Dump => handle_ctrl(CtrlSignal::Backtrace(BackTraceCommand::Show {
            cc: true,
            python: true,
            tid: None,
        })),
        CtrlSignal::Dap { address } => handle_ctrl(Enable(Features::Dap { address })),
        CtrlSignal::Pause { address } => todo!(),
        CtrlSignal::Perf => handle_ctrl(Enable(Features::Pprof)),
        CtrlSignal::CatchCrash => handle_ctrl(Enable(Features::CatchCrash { address: None })),
        CtrlSignal::ListenRemote { address } => handle_ctrl(Enable(Features::Remote { address })),
        CtrlSignal::Execute { script } => handle_ctrl(CtrlSignal::Eval { code: script }),
        CtrlSignal::ShowPLT => handle_ctrl(Show(ShowCommand::PLT)),

        CtrlSignal::Enable(feature) => enable::handle(feature),
        CtrlSignal::Disable(feature) => disable::handle(feature),
        CtrlSignal::Show(topic) => show::handle(topic),
        CtrlSignal::Backtrace(bt) => backtrace::handle(bt),
        CtrlSignal::Eval { code } => eval::handle(code),
    }
}

pub fn not_implemented() -> Result<String> {
    anyhow::bail!("not implemented!")
}
