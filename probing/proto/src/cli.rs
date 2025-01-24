// #[cfg(feature = "cli")]
// use clap::{Parser, Subcommand};
// use serde::{Deserialize, Serialize};

// #[cfg_attr(feature = "cli", derive(Subcommand))]
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum BackTraceCommand {
//     /// show backtrace
//     Show {
//         /// enable C/C++ backtrace
//         #[cfg_attr(feature = "cli", arg(long))]
//         cc: bool,

//         /// enable Python backtrace
//         #[cfg_attr(feature = "cli", arg(long))]
//         python: bool,

//         /// target thread id, default is the main thread
//         #[cfg_attr(feature = "cli", arg(short, long))]
//         tid: Option<u64>,
//     },

//     /// pause the target process and start a debug server
//     Pause {
//         #[cfg_attr(feature = "cli", arg(short, long))]
//         address: Option<String>,

//         #[cfg_attr(feature = "cli", arg(short, long))]
//         tid: Option<u32>,

//         #[cfg_attr(
//             feature = "cli",
//             arg(short, long, hide = true, default_value = "false")
//         )]
//         signal: bool,
//     },

//     #[cfg_attr(feature = "cli", command(hide = true))]
//     Trigger {
//         #[cfg_attr(feature = "cli", arg(long))]
//         cc: bool,
//         #[cfg_attr(feature = "cli", arg(long))]
//         python: bool,
//     },
// }

// #[cfg_attr(feature = "cli", derive(Subcommand))]
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum TraceCommand {
//     #[cfg_attr(feature = "cli", command(visible_aliases=["py"]))]
//     Python { function: String, watch: String },

//     #[cfg_attr(feature = "cli", command(visible_aliases=["c"]))]
//     Clear { function: String },

//     #[cfg_attr(feature = "cli", command(visible_aliases=["all"]))]
//     Show,
// }
