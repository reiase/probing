use anyhow::Result;
use argh::FromArgs;

use nix::{sys::signal, unistd::Pid};

/// Dump the calling stack of the target process
#[derive(FromArgs)]
#[argh(subcommand, name = "dump")]
pub struct DumpCommand {}

impl DumpCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR2)
            .map_err(|err| anyhow::anyhow!("error sending signal to pid {pid}: {}", err.desc()))
    }
}
