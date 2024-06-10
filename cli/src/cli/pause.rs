use anyhow::Result;
use argh::FromArgs;

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
        let argstr = if let Some(addr) = &self.address {
            format!(" -p -a {addr}")
        } else {
            " -p".to_string()
        };
        usr1_handler(argstr, pid)
    }
}
