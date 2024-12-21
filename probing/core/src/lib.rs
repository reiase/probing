use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

pub use probing_proto::protocol::process::CallFrame;
pub use probing_proto::protocol::process::Value;

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ProbeCallProtocol {
    CallBacktrace(Option<i32>),
    ReturnBacktrace(Vec<CallFrame>),
    CallEval(String),
    ReturnEval(String),
    Err(String),
}

pub trait Probe: Send + Sync {
    fn backtrace(&self, depth: Option<i32>) -> Result<Vec<CallFrame>>;
    fn eval(&self, code: &str) -> Result<String>;

    fn handle(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let msg = ron::de::from_bytes::<ProbeCallProtocol>(msg)?;
        let res = match msg {
            ProbeCallProtocol::CallBacktrace(depth) => match self.backtrace(depth) {
                Ok(res) => ProbeCallProtocol::ReturnBacktrace(res),
                Err(err) => ProbeCallProtocol::Err(err.to_string()),
            },
            ProbeCallProtocol::CallEval(code) => match self.eval(&code) {
                Ok(res) => ProbeCallProtocol::ReturnEval(res),
                Err(err) => ProbeCallProtocol::Err(err.to_string()),
            },
            ProbeCallProtocol::Err(err) => ProbeCallProtocol::Err(err),
            _ => unreachable!(),
        };
        Ok(ron::to_string(&res)?.as_bytes().to_vec())
    }
}

pub trait ProbeFactory: Send + Sync {
    fn create(&self) -> Arc<dyn Probe>;
}

pub mod ccprobe;
