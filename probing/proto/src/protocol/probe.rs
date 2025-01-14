use std::fmt::Display;
use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::protocol::process::CallFrame;

#[cfg_attr(feature = "actor", derive(actix::Message))]
#[cfg_attr(feature = "actor", rtype(result = "ProbeCall"))]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ProbeCall {
    CallEnable(String),
    ReturnEnable(()),

    CallDisable(String),
    ReturnDisable(()),

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
            ProbeCall::CallEnable(feature) => write!(f, "CallEnable({:?})", feature),
            ProbeCall::ReturnEnable(res) => write!(f, "ReturnEnable({:?})", res),

            ProbeCall::CallDisable(feature) => write!(f, "CallDisable({:?})", feature),
            ProbeCall::ReturnDisable(()) => write!(f, "ReturnDisable(()"),

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

#[cfg(feature = "actor")]
impl<A, M> actix::dev::MessageResponse<A, M> for ProbeCall
where
    A: actix::Actor,
    M: actix::Message<Result = ProbeCall>,
{
    fn handle(self, _ctx: &mut A::Context, tx: Option<actix::dev::OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            let _ = tx.send(self);
        }
    }
}

pub trait Probe: Send + Sync {
    fn enable(&self, feture: &str) -> Result<()>;
    fn disable(&self, feture: &str) -> Result<()>;

    fn backtrace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>>;
    fn eval(&self, code: &str) -> Result<String>;

    fn flamegraph(&self) -> Result<String> {
        Err(anyhow::anyhow!("not implemented"))
    }

    fn handle(&self, msg: &[u8]) -> Result<Vec<u8>> {
        let msg = ron::de::from_bytes::<ProbeCall>(msg)?;
        log::debug!("probe request: {}", msg);
        let res = match msg {
            ProbeCall::CallEnable(feature) => match self.enable(&feature) {
                Ok(res) => ProbeCall::ReturnEnable(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
            ProbeCall::CallDisable(feature) => match self.disable(&feature) {
                Ok(res) => ProbeCall::ReturnDisable(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
            ProbeCall::CallBacktrace(depth) => match self.backtrace(depth) {
                Ok(res) => ProbeCall::ReturnBacktrace(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
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
        };
        log::debug!("probe reply: {}", res);
        Ok(ron::to_string(&res)?.as_bytes().to_vec())
    }
}

pub trait ProbeFactory: Send + Sync {
    fn create(&self) -> Arc<dyn Probe>;
}
