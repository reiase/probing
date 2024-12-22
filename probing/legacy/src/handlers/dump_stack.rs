use anyhow::{Context, Result};
use log::info;
use nix::unistd::{getpid, gettid};
use probing_proto::CallStack;
use pyo3::{types::PyAnyMethods, Python, ToPyObject};

use crate::repl::PythonRepl;

pub fn dump_stack() -> Result<String> {
    let tid = gettid();
    let pid = getpid();
    info!("call stack dump from tid: {} in pid: {}", tid, pid);
    let mut repl = PythonRepl::default();
    let request = "dump_stack()".to_string();
    let ret = repl
        .process(request.as_str())
        .ok_or(anyhow::anyhow!("dump stack failed"))?;
    let mut ret = serde_json::from_str::<Vec<CallStack>>(ret.as_str())
        .with_context(|| "error deserializing dump stack result".to_string())?;
    ret.insert(
        0,
        CallStack {
            cstack: Some(cc_backtrace()),
            ..Default::default()
        },
    );
    let ret = serde_json::to_string(&ret)?;
    Ok(ret)
}

pub fn dump_stack2() {
    Python::with_gil(|_| {
        let mut ret = String::new();
        ret.push_str(py_backtrace().as_str());
        ret.push_str(cc_backtrace().as_str());
        eprintln!("{}", ret);
    });
}

pub fn py_backtrace() -> String {
    Python::with_gil(|py| {
        let ret = py
            .import("traceback")
            .unwrap()
            .call_method0("format_stack")
            .unwrap_or_else(|err| {
                err.print(py);
                err.to_string().to_object(py).into_bound(py)
            });
        let ret = "\n"
            .to_object(py)
            .call_method1(py, "join", (ret.as_unbound(),));
        match ret {
            Ok(obj) => obj.to_string(),
            Err(err) => {
                err.print(py);
                err.to_string()
            }
        }
    })
}

pub fn cc_backtrace() -> String {
    let mut ret = String::new();
    let mut cnt = 0;
    backtrace::trace(|frame| {
        let ip = frame.ip() as usize;
        ret.push_str(format!("frame #{:<2} - {:#02$x}:\n", cnt, ip, 20).as_str());
        cnt += 1;
        backtrace::resolve_frame(frame, |symbol| {
            let name = symbol
                .name()
                .map(|n| n.to_string())
                .unwrap_or("<unknown>".to_string());
            let filename = symbol.filename().map(|f| f.to_string_lossy().to_string());
            let lineno = symbol.lineno();
            let func = if let (Some(filename), Some(lineno)) = (filename, lineno) {
                format!("\t{name}\n\t  at {filename:13} : {lineno:4}\n",)
            } else {
                format!("\t{name}\n")
            };
            ret.push_str(func.as_str());
        });
        true // keep going
    });
    ret
}
