use argh::FromArgs;
use anyhow::Result;

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
        let argstr = if let Some(addr) = &self.address {
            format!(" -b -a {}", addr)
        } else {
            " -b".to_string()
        };
        usr1_handler(argstr, pid)
    }
}
