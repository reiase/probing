use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod cli;
pub mod protocol;
pub mod types;

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Process {
    pub pid: i32,
    pub exe: String,
    pub env: String,
    pub cmd: String,
    pub cwd: String,
    pub main_thread: u64,
    pub threads: Vec<u64>,
}

#[derive(Clone)]
pub struct KeyValuePair {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct CallStack {
    pub cstack: Option<String>,
    pub file: String,
    pub func: String,
    pub lineno: i64,
    pub locals: HashMap<String, Object>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Object {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct DebugState {
    pub debugger_installed: bool,
    pub debugger_address: Option<String>,
}

pub mod prelude {
    pub use crate::protocol::query::Format as QueryDataFormat;
    pub use crate::protocol::query::Message as QueryMessage;
    pub use crate::protocol::query::Options as QueryOptions;
    pub use crate::protocol::query::Query as QueryRequest;
    pub use crate::protocol::query::Reply as QueryReply;

    pub use crate::protocol::cluster::Cluster;
    pub use crate::protocol::cluster::Node;

    pub use crate::types::DataFrame;
    pub use crate::types::Table;

    pub use crate::protocol::probe::Probe;
    pub use crate::protocol::probe::ProbeCall;
    pub use crate::protocol::probe::ProbeFactory;
}
