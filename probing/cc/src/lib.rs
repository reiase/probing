mod plugins;

pub use probing_proto::protocol::process::CallFrame;
pub use probing_proto::protocol::process::Value;

pub use probing_proto::prelude::Probe;
pub use probing_proto::prelude::ProbeCallProtocol;
pub use probing_proto::prelude::ProbeFactory;

pub mod ccprobe;
