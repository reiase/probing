pub mod protocol;
pub mod types;

pub mod prelude {
    // --- Protocol Structures ---
    pub use crate::protocol::cluster::{Cluster, Node};
    pub use crate::protocol::message::Message;
    pub use crate::protocol::process::{CallFrame, Process};

    pub use crate::protocol::query::{Data as QueryDataFormat, Options as QueryOptions, Query};
    pub use crate::protocol::query::{QueryError, ErrorCode};
    pub use crate::protocol::version::ProtocolVersion;

    // --- Core Data Types ---
    pub use crate::types::DataFrame;
    pub use crate::types::Ele;
    pub use crate::types::Seq;
    pub use crate::types::Series;
    pub use crate::types::TimeSeries;
    pub use crate::types::Value;

    // --- Error Handling ---
    pub use crate::types::ProtoError;
}
