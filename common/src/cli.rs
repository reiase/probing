use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Serialize, Deserialize, Debug, Clone)]
pub enum CtrlSignal {
    Nil,
    Dump,
    Dap {
        address: Option<String>,
    },
    Pause {
        address: Option<String>,
    },
    Perf,
    CatchCrash,
    ListenRemote {
        address: Option<String>,
    },
    Execute {
        script: String,
    },
    ShowPLT,

    #[command(subcommand, aliases = ["bt"])]
    BackTrace(BackTraceCommand),

    #[command(subcommand)]
    Show(TopicCommand),
}

#[derive(Subcommand, Serialize, Deserialize, Debug, Clone)]
pub enum BackTraceCommand {
    Show {
        #[arg(long)]
        cc: bool,
        #[arg(long)]
        python: bool,
        #[arg(short, long)]
        tid: Option<u64>,
    },
}

#[derive(Subcommand, Serialize, Deserialize, Debug, Clone)]
pub enum TopicCommand {
    #[command()]
    Memory,
    #[command()]
    Threads,
    #[command()]
    Objects,
    #[command()]
    Tensors,
    #[command()]
    Modules,
}
