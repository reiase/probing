use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProbeCommand {
    Nil,
    Dump,
    Pause { address: Option<String> },
    Pprof,
    CatchCrash,
    ListenRemote { address: Option<String> },
    Execute { script: String },
}
