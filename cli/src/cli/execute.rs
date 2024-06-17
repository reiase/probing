use anyhow::Result;
use argh::FromArgs;
use probe_common::cli::ProbeCommand;

use super::usr1_handler;

/// Execute a script in the target process
#[derive(FromArgs)]
#[argh(subcommand, name = "exec")]
pub struct ExecuteCommand {
    /// script to execute (e.g., /path/to/script.py)
    #[argh(positional)]
    script: String,
}

impl ExecuteCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        let probe_command = ProbeCommand::Execute {
            script: self.script.clone(),
        };
        usr1_handler(ron::to_string(&probe_command).unwrap(), pid)
    }
}
