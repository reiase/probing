use anyhow::Result;
use argh::FromArgs;

use nix::{sys::signal, unistd::Pid};

/// Start profiling
#[derive(FromArgs)]
#[argh(subcommand, name = "pprof")]
pub struct PprofCommand {}

impl PprofCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        signal::kill(Pid::from_raw(pid), signal::Signal::SIGPROF)
            .map_err(|err| anyhow::anyhow!("error sending signal to pid {pid}: {}", err.desc()))
    }
}
