
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Serialize, Deserialize, Debug, Clone)]
pub enum CtrlSignal {
    #[command(hide = true)]
    Nil,
    #[command(hide = true)]
    Dump,

    #[command(subcommand)]
    Enable(Features),

    #[command(subcommand)]
    Disable(Features),

    #[command(subcommand)]
    Show(ShowCommand),

    #[command(subcommand, visible_aliases = ["bt"])]
    Backtrace(BackTraceCommand),

    #[command()]
    Eval {
        #[arg()]
        code: String,
    },
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

    Pause {
        #[arg(short, long)]
        address: Option<String>,

        #[arg(short, long)]
        tid: Option<u32>,

        #[arg(hide = true, default_value = "false")]
        signal: bool,
    },

    #[command(hide = true)]
    Trigger {
        #[arg(long)]
        cc: bool,
        #[arg(long)]
        python: bool,
    },
}

#[derive(Subcommand, Serialize, Deserialize, Debug, Clone)]
pub enum ShowCommand {
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
    #[command()]
    PLT,
}

#[derive(Subcommand, Serialize, Deserialize, Debug, Clone)]
pub enum Features {
    #[command()]
    Pprof,

    #[command()]
    Dap { address: Option<String> },

    #[command()]
    Remote { address: Option<String> },

    #[command()]
    CatchCrash { address: Option<String> },
}
