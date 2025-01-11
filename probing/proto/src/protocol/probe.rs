use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::protocol::process::CallFrame;

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ProbeCallProtocol {
    CallBacktrace(Option<i32>),
    ReturnBacktrace(Vec<CallFrame>),
    CallEval(String),
    ReturnEval(String),
    CallEnable(String),
    ReturnEnable(()),
    Nil,
    Err(String),
}

impl Display for ProbeCallProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeCallProtocol::CallBacktrace(depth) => write!(f, "CallBacktrace({:?})", depth),
            ProbeCallProtocol::ReturnBacktrace(frames) => {
                write!(f, "ReturnBacktrace({:?})", frames)
            }
            ProbeCallProtocol::CallEval(code) => write!(f, "CallEval({:?})", code),
            ProbeCallProtocol::ReturnEval(res) => write!(f, "ReturnEval({:?})", res),
            ProbeCallProtocol::CallEnable(feature) => write!(f, "CallEnable({:?})", feature),
            ProbeCallProtocol::ReturnEnable(res) => write!(f, "ReturnEnable({:?})", res),
            ProbeCallProtocol::Nil => write!(f, "Nil"),
            ProbeCallProtocol::Err(err) => write!(f, "Err({:?})", err),
        }
    }
}

pub trait Probe: Send + Sync {
    fn backtrace(&self, depth: Option<i32>) -> Result<Vec<CallFrame>>;
    fn eval(&self, code: &str) -> Result<String>;
    fn enable(&self, feture: &str) -> Result<()>;

    fn handle(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let msg = ron::de::from_bytes::<ProbeCallProtocol>(msg)?;
        log::debug!("probe request: {}", msg);
        let res = match msg {
            ProbeCallProtocol::CallBacktrace(depth) => match self.backtrace(depth) {
                Ok(res) => ProbeCallProtocol::ReturnBacktrace(res),
                Err(err) => ProbeCallProtocol::Err(err.to_string()),
            },
            ProbeCallProtocol::CallEval(code) => match self.eval(&code) {
                Ok(res) => ProbeCallProtocol::ReturnEval(res),
                Err(err) => ProbeCallProtocol::Err(err.to_string()),
            },
            ProbeCallProtocol::CallEnable(feature) => match self.enable(&feature) {
                Ok(res) => ProbeCallProtocol::ReturnEnable(res),
                Err(err) => ProbeCallProtocol::Err(err.to_string()),
            },
            ProbeCallProtocol::Err(err) => ProbeCallProtocol::Err(err),
            _ => unreachable!(),
        };
        log::debug!("probe reply: {}", res);
        Ok(ron::to_string(&res)?.as_bytes().to_vec())
    }
}

pub trait ProbeFactory: Send + Sync {
    fn create(&self) -> Arc<dyn Probe>;
}
