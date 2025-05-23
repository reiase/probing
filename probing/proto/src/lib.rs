use serde::{Deserialize, Serialize};

pub mod protocol;
pub mod types;

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct DebugState {
    pub debugger_installed: bool,
    pub debugger_address: Option<String>,
}

pub mod prelude {
    pub use crate::protocol::query::Data as QueryDataFormat;
    pub use crate::protocol::query::Query;
    // pub use crate::protocol::query::QueryMessage as QueryMessage;
    pub use crate::protocol::query::Options as QueryOptions;

    pub use crate::protocol::cluster::Cluster;
    pub use crate::protocol::cluster::Node;

    pub use crate::protocol::process::Process;

    pub use crate::types::DataFrame;

    pub use crate::protocol::message::Message;
    pub use crate::protocol::version::ProtocolVersion;
}
