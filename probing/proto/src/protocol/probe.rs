use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::protocol::process::CallFrame;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ProbeCall {
    CallBacktrace(Option<i32>),
    ReturnBacktrace(Vec<CallFrame>),

    CallEval(String),
    ReturnEval(String),

    CallFlamegraph,
    ReturnFlamegraph(String),

    Nil,
    Err(String),
}

impl Display for ProbeCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeCall::CallBacktrace(depth) => write!(f, "CallBacktrace({:?})", depth),
            ProbeCall::ReturnBacktrace(frames) => {
                write!(f, "ReturnBacktrace({:?})", frames)
            }
            ProbeCall::CallEval(code) => write!(f, "CallEval({:?})", code),
            ProbeCall::ReturnEval(res) => write!(f, "ReturnEval({:?})", res),

            ProbeCall::CallFlamegraph => write!(f, "CallFlamegraph"),
            ProbeCall::ReturnFlamegraph(res) => write!(f, "ReturnFlamegraph({:?})", res),

            ProbeCall::Nil => write!(f, "Nil"),
            ProbeCall::Err(err) => write!(f, "Err({:?})", err),
        }
    }
}

pub trait Probe: Send + Sync {
    // fn backtrace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>>;
    fn eval(&self, code: &str) -> Result<String>;

    fn flamegraph(&self) -> Result<String> {
        Err(anyhow::anyhow!("not implemented"))
    }

    fn ask(&self, request: ProbeCall) -> ProbeCall {
        match request {
            // ProbeCall::CallBacktrace(depth) => match self.backtrace(depth) {
            //     Ok(res) => ProbeCall::ReturnBacktrace(res),
            //     Err(err) => ProbeCall::Err(err.to_string()),
            // },
            ProbeCall::CallEval(code) => match self.eval(&code) {
                Ok(res) => ProbeCall::ReturnEval(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
            ProbeCall::CallFlamegraph => match self.flamegraph() {
                Ok(res) => ProbeCall::ReturnFlamegraph(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
            ProbeCall::Err(err) => ProbeCall::Err(err),
            _ => unreachable!(),
        }
    }

    fn handle(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let msg = serde_json::de::from_slice::<ProbeCall>(msg)?;
        log::debug!("probe request: {}", msg);
        let res = self.ask(msg);
        log::debug!("probe reply: {}", res);
        Ok(serde_json::to_string(&res)?.as_bytes().to_vec())
    }
}

pub trait ProbeFactory: Send + Sync {
    fn create(&self) -> Arc<dyn Probe>;
}
