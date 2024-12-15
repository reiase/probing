use std::sync::Arc;

use anyhow::Result;

pub use probing_proto::protocol::process::CallFrame;
pub use probing_proto::protocol::process::Value;

pub trait Probe: Send + Sync{
    fn backtrace(&self, depth: Option<i32>) -> Result<Vec<CallFrame>>;
    fn eval(&self, code: &str) -> Result<String>;
}

pub trait ProbeFactory: Send + Sync{
    fn create(&self) -> Arc<dyn Probe>;
}

pub mod ccprobe;