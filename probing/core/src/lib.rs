use anyhow::Result;

pub use probing_proto::protocol::process::CallFrame;
pub use probing_proto::protocol::process::Value;

pub trait Probe {
    fn backtrace(depth: Option<i32>) -> Result<Vec<CallFrame>>;
    fn eval<T: Into<String>>(code: T) -> Result<String>;
}