#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "cli", derive(Parser))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CtrlSignal {
    #[cfg_attr(feature = "cli", command(hide = true))]
    Nil,
    #[cfg_attr(feature = "cli", command(hide = true))]
    Dump,

    #[cfg_attr(feature = "cli", command(subcommand))]
    Enable(Features),

    #[cfg_attr(feature = "cli", command(subcommand))]
    Disable(Features),

    #[cfg_attr(feature = "cli", command(subcommand))]
    Show(ShowCommand),

    #[cfg_attr(feature = "cli", command(subcommand, visible_aliases = ["bt"]))]
    Backtrace(BackTraceCommand),

    #[cfg_attr(feature = "cli", command(subcommand))]
    Trace(TraceCommand),

    #[cfg_attr(feature = "cli", command())]
    Eval {
        #[cfg_attr(feature = "cli", arg())]
        code: String,
    },
}

#[cfg_attr(feature = "cli", derive(Subcommand))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BackTraceCommand {
    /// show backtrace
    Show {
        /// enable C/C++ backtrace
        #[cfg_attr(feature = "cli", arg(long))]
        cc: bool,

        /// enable Python backtrace
        #[cfg_attr(feature = "cli", arg(long))]
        python: bool,

        /// target thread id, default is the main thread
        #[cfg_attr(feature = "cli", arg(short, long))]
        tid: Option<u64>,
    },

    /// pause the target process and start a debug server
    Pause {
        #[cfg_attr(feature = "cli", arg(short, long))]
        address: Option<String>,

        #[cfg_attr(feature = "cli", arg(short, long))]
        tid: Option<u32>,

        #[cfg_attr(feature = "cli", arg(short, long, hide = true, default_value = "false"))]
        signal: bool,
    },

    #[cfg_attr(feature = "cli", command(hide = true))]
    Trigger {
        #[cfg_attr(feature = "cli", arg(long))]
        cc: bool,
        #[cfg_attr(feature = "cli", arg(long))]
        python: bool,
    },
}

#[cfg_attr(feature = "cli", derive(Subcommand))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShowCommand {
    /// show memory usages
    #[cfg_attr(feature = "cli", command())]
    Memory,

    /// show threads
    #[cfg_attr(feature = "cli", command())]
    Threads,

    /// show python objects
    #[cfg_attr(feature = "cli", command())]
    Objects,

    /// show torch tensors
    #[cfg_attr(feature = "cli", command())]
    Tensors,

    /// show torch modules
    #[cfg_attr(feature = "cli", command())]
    Modules,

    /// show traceable functions
    #[cfg_attr(feature = "cli", command())]
    Traceable { filter: Option<String> },

    /// show hookable C functions
    #[cfg_attr(feature = "cli", command())]
    PLT,

    /// read information from ffi function [()->char*] call, e.g. `get_current_dir_name()`
    #[cfg_attr(feature = "cli", command())]
    FFI { name: String },
}

#[cfg_attr(feature = "cli", derive(Subcommand))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Features {
    /// pprof-like performance data visualization
    #[cfg_attr(feature = "cli", command())]
    Pprof,

    /// debug python with DAP (debug adapter protocol)
    #[cfg_attr(feature = "cli", command())]
    Dap { address: Option<String> },

    /// remote control the target process
    #[cfg_attr(feature = "cli", command())]
    Remote { address: Option<String> },

    /// catch process crash and start a server for remote debugging
    #[cfg_attr(feature = "cli", command())]
    CatchCrash { address: Option<String> },
}

#[cfg_attr(feature = "cli", derive(Subcommand))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TraceCommand {
    #[cfg_attr(feature = "cli", command(visible_aliases=["py"]))]
    Python { function: String, watch: String },

    #[cfg_attr(feature = "cli", command(visible_aliases=["c"]))]
    Clear { function: String },

    #[cfg_attr(feature = "cli", command(visible_aliases=["all"]))]
    Show,
}
