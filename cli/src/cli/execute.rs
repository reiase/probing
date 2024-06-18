use anyhow::Result;
use clap::Args;
use probe_common::cli::ProbeCommand;

use super::usr1_handler;

/// Execute a script in the target process
#[derive(Args)]
#[command(version, about, long_about = None)]
pub struct ExecuteCommand {
    /// script to execute (e.g., /path/to/script.py)
    #[arg()]
    script: String,
}

impl ExecuteCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        let cmd = ProbeCommand::Execute {
            script: self.script.clone(),
        };
        let cmd = ron::to_string(&cmd)?;
        usr1_handler(cmd, pid)
    }
}
