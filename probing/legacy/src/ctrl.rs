use std::{fmt::Display, sync::Arc};

use anyhow::Result;
use probing_engine::plugins::cluster::ClusterPlugin;
use probing_proto::cli::{BackTraceCommand, CtrlSignal};
use probing_proto::protocol::query::{Format, Message, Reply};

// use crate::handlers::dump_stack;
// use crate::service::CALLSTACK;

use probing_python::plugins::python::PythonPlugin;

pub fn ctrl_handler(cmd: CtrlSignal) -> Result<()> {
    match cmd {
        CtrlSignal::Nil => {}
        CtrlSignal::Dump => {
            // let ret = dump_stack()?;
            // CALLSTACK
            //     .lock()
            //     .map(|mut cs| {
            //         cs.replace(ret);
            //     })
            //     .unwrap();
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
        if let Ok(cmd) = ron::from_str::<CtrlSignal>(&cmdstr) {
            let _ = ctrl_handler(cmd).map_err(|err| eprintln!("{}", err));
            return;
        } else if let Ok(msg) = ron::from_str::<Message>(&cmdstr) {
            let _ = handle_query(msg).map_err(|err| eprintln!("{}", err));
            return;
        }
        let cmd: CtrlSignal = ron::from_str(&cmdstr).unwrap_or(CtrlSignal::Nil);
        let _ = ctrl_handler(cmd).map_err(|err| eprintln!("{}", err));
    }
}

// mod backtrace;
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

pub fn handle_ctrl(ctrl: CtrlSignal) -> Result<Vec<u8>> {
    match ctrl {
        CtrlSignal::Nil => Ok(Default::default()),
        CtrlSignal::Dump => handle_ctrl(CtrlSignal::Backtrace(BackTraceCommand::Show {
            cc: true,
            python: true,
            tid: None,
        })),
        CtrlSignal::Enable(feature) => enable::handle(feature).map(|x| x.into_bytes()),
        CtrlSignal::Disable(feature) => disable::handle(feature).map(|x| x.into_bytes()),
        CtrlSignal::Show(topic) => show::handle(topic).map(|x| x.into_bytes()),
        CtrlSignal::Backtrace(bt) => {todo!()},//backtrace::handle(bt).map(|x| x.into_bytes()),
        CtrlSignal::Trace(cmd) => trace::handle(cmd).map(|x| x.into_bytes()),
        CtrlSignal::Eval { code } => eval::handle(code).map(|x| x.into_bytes()),
        // CtrlSignal::Query { query } => {
        //     let engine = probing_engine::create_engine();
        //     engine.enable("probe", Arc::new(PythonPlugin::new("python")))?;
        //     engine.enable("probe", Arc::new(ClusterPlugin::new("nodes", "cluster")))?;
        //     engine.execute(query.as_str(), "ron")
        // }
    }
}

pub fn handle_query(query: Message) -> Result<Vec<u8>> {
    let engine = probing_engine::create_engine();
    engine.enable("probe", Arc::new(PythonPlugin::new("python")))?;
    engine.enable("probe", Arc::new(ClusterPlugin::new("nodes", "cluster")))?;
    if let Message::Query(query) = query {
        let resp = engine.execute(&query.expr, "ron")?;
        Ok(ron::to_string(&Message::Reply(Reply {
            data: resp,
            format: Format::RON,
        }))?
        .as_bytes()
        .to_vec())
    } else {
        Err(anyhow::anyhow!("Invalid query message"))
    }
}
