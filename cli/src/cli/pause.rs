use anyhow::Result;
use argh::FromArgs;
use probe_common::cli::ProbeCommand;

use super::usr1_handler;

/// Pause the target process and listen for remote connection
#[derive(FromArgs)]
#[argh(subcommand, name = "pause")]
pub struct PauseCommand {
    /// address to listen
    #[argh(option, short = 'a')]
    address: Option<String>,
}

impl PauseCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        let probe_command = ProbeCommand::Pause {
            address: self.address.clone(),
        };
        usr1_handler(ron::to_string(&probe_command).unwrap(), pid)
    }
}
