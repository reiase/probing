use serde::{Deserialize, Serialize};

pub mod protocol;
pub mod types;

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
    pub use crate::protocol::query::Data as QueryDataFormat;
    pub use crate::protocol::query::Message as QueryMessage;
    pub use crate::protocol::query::Options as QueryOptions;

    pub use crate::protocol::cluster::Cluster;
    pub use crate::protocol::cluster::Node;

    pub use crate::protocol::process::Process;

    pub use crate::types::DataFrame;
    pub use crate::types::Table;

    pub use crate::protocol::probe::Probe;
    pub use crate::protocol::probe::ProbeCall;
    pub use crate::protocol::probe::ProbeFactory;
}
