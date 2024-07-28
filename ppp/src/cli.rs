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
    /// show backtrace
    Show {
        /// enable C/C++ backtrace
        #[arg(long)]
        cc: bool,

        /// enable Python backtrace
        #[arg(long)]
        python: bool,

        /// target thread id, default is the main thread
        #[arg(short, long)]
        tid: Option<u64>,
    },

    /// pause the target process and start a debug server
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
    /// show memory usages
    #[command()]
    Memory,

    /// show threads
    #[command()]
    Threads,

    /// show python objects
    #[command()]
    Objects,

    /// show torch tensors
    #[command()]
    Tensors,

    /// show torch modules
    #[command()]
    Modules,

    /// show traceable functions
    #[command()]
    Traceable {filter: Option<String> },

    /// show hookable C functions
    #[command()]
    PLT,

    /// read information from ffi function [()->char*] call, e.g. `get_current_dir_name()`
    #[command()]
    FFI { name: String },
}

#[derive(Subcommand, Serialize, Deserialize, Debug, Clone)]
pub enum Features {
    /// pprof-like performance data visualization
    #[command()]
    Pprof,

    /// debug python with DAP (debug adapter protocol)
    #[command()]
    Dap { address: Option<String> },

    /// remote control the target process
    #[command()]
    Remote { address: Option<String> },

    /// catch process crash and start a server for remote debugging
    #[command()]
    CatchCrash { address: Option<String> },
}
