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
        let probe_command = ProbeCommand::ListenRemote {
            address: self.address.clone(),
        };
        usr1_handler(ron::to_string(&probe_command).unwrap(), pid)
    }
}
