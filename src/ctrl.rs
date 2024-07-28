use std::fmt::Display;

use anyhow::Result;
use probing_ppp::cli::{BackTraceCommand, CtrlSignal};

use crate::{handlers::dump_stack, service::CALLSTACK};

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
mod trace;

#[derive(Default)]
pub struct StringBuilder {
    buf: String,
}

impl Display for StringBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buf)
    }
}

pub trait StringBuilderAppend {
    fn append_line(&self, builder: &mut StringBuilder);
}

impl StringBuilderAppend for String {
    fn append_line(&self, builder: &mut StringBuilder) {
        builder.buf.push_str(self.as_str());
        builder.buf.push('\n');
    }
}

impl StringBuilderAppend for &str {
    fn append_line(&self, builder: &mut StringBuilder) {
        builder.buf.push_str(self);
        builder.buf.push('\n');
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
        CtrlSignal::Enable(feature) => enable::handle(feature),
        CtrlSignal::Disable(feature) => disable::handle(feature),
        CtrlSignal::Show(topic) => show::handle(topic),
        CtrlSignal::Backtrace(bt) => backtrace::handle(bt),
        CtrlSignal::Trace(cmd) => trace::handle(cmd),
        CtrlSignal::Eval { code } => eval::handle(code),
    }
}
