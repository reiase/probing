use anyhow::Context;
use anyhow::Ok;
use anyhow::Result;

use clap::Args;
use nix::{sys::signal, unistd::Pid};
use probing_common::cli::ProbingCommand;

use super::send_ctrl;

/// Debug and Inspection Tool
#[derive(Args)]
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
    pub fn run(&self, pid: i32) -> Result<()> {
        if self.dump {
            signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR2)
                .with_context(|| format!("error sending signal to pid {pid}"))
        } else if self.pause {
            let cmd = ProbingCommand::Pause {
                address: self.address.clone(),
            };
            let cmd = ron::to_string(&cmd)?;
            send_ctrl(cmd, pid)
        } else if self.dap {
            let cmd = ProbingCommand::Dap {
                address: self.address.clone(),
            };
            let cmd = ron::to_string(&cmd)?;
            send_ctrl(cmd, pid)
        } else {
            Ok(())
        }
    }
}
