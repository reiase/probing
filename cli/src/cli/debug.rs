use anyhow::Context;
use anyhow::Ok;
use anyhow::Result;

use clap::Args;
use nix::{sys::signal, unistd::Pid};
use probe_common::cli::ProbeCommand;

use super::usr1_handler;

/// Debug the target process
#[derive(Args)]
pub struct DebugCommand {
    /// Dump the calling stack of the target process
    #[arg(short, long, conflicts_with_all=["pause"])]
    dump: bool,

    /// Pause the target process and listen for remote connection
    #[arg(short, long, conflicts_with_all=["dump"])]
    pause: bool,

    /// address to listen
    #[arg(short, long, default_value = "127.0.0.1:9922")]
    address: Option<String>,
}

impl DebugCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        if self.dump {
            signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR2)
                .with_context(|| format!("error sending signal to pid {pid}"))
        } else if self.pause {
            let cmd = ProbeCommand::Pause {
                address: self.address.clone(),
            };
            let cmd = ron::to_string(&cmd)?;
            usr1_handler(cmd, pid)
        } else {
            Ok(())
        }
    }
}
