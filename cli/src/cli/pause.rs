use anyhow::Result;
use clap::Args;
use probe_common::cli::ProbeCommand;

use super::usr1_handler;

/// Pause the target process and listen for remote connection
#[derive(Args, Default)]
#[command(version, about, long_about = None)]
pub struct PauseCommand {
    /// address to listen
    #[arg(short, long, default_value = "127.0.0.1:9922")]
    address: Option<String>,
}

impl PauseCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        let cmd = ProbeCommand::Pause {
            address: self.address.clone(),
        };
        let cmd = ron::to_string(&cmd)?;
        usr1_handler(cmd, pid)
    }
}
