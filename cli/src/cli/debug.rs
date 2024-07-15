use anyhow::Result;

use clap::Args;
use probing_common::cli::ProbingCommand;

use super::ctrl::CtrlChannel;

/// Debug and Inspection Tool
#[derive(Args, Debug)]
pub struct DebugCommand {
    /// Dump the calling stack of the target process
    #[arg(short, long, conflicts_with_all=["dap", "pause"])]
    dump: bool,

    /// Pause the target process and listen for remote connection
    #[arg(short, long, conflicts_with_all=["dump", "dap"])]
    pause: bool,

    /// Start DAP server and debugging python code from vscode
    #[arg(long, conflicts_with_all = ["pause", "dump"])]
    dap: bool,

    /// address to listen when using `pause` or `dap`
    #[arg(short, long, default_value = None)]
    address: Option<String>,
}

impl DebugCommand {
    pub fn run(&self, ctrl: CtrlChannel) -> Result<()> {
        let cmd = if self.dump {
            ProbingCommand::Dump
        } else if self.pause {
            ProbingCommand::Pause {
                address: self.address.clone(),
            }
        } else if self.dap {
            ProbingCommand::Dap {
                address: self.address.clone(),
            }
        } else {
            ProbingCommand::Nil
        };
        let cmd = ron::to_string(&cmd)?;
        ctrl.send_ctrl(cmd)
    }
}
