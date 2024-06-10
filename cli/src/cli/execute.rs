use anyhow::Result;
use argh::FromArgs;

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
        usr1_handler(format!(" -e {}", self.script), pid)
    }
}
