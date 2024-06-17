use anyhow::Result;
use argh::FromArgs;
use probe_common::cli::ProbeCommand;

use super::usr1_handler;

/// Start background server and listen for remote connections
#[derive(FromArgs)]
#[argh(subcommand, name = "listen")]
pub struct ListenRemoteCommand {
    /// address to listen
    #[argh(positional)]
    address: Option<String>,
}

impl ListenRemoteCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        let cmd = ProbeCommand::ListenRemote {
            address: self.address.clone(),
        };
        let cmd = ron::to_string(&cmd)?;
        usr1_handler(cmd, pid)
    }
}
