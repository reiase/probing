use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProbingCommand {
    Nil,
    Dump,
    Dap {address: Option<String>},
    Pause { address: Option<String> },
    Perf,
    CatchCrash,
    ListenRemote { address: Option<String> },
    Execute { script: String },
    ShowPLT,
}
