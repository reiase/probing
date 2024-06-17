use anyhow::{Context, Result};
use argh::FromArgs;

use nix::{sys::signal, unistd::Pid};

/// Start profiling
#[derive(FromArgs)]
#[argh(subcommand, name = "pprof")]
pub struct PprofCommand {}

impl PprofCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        signal::kill(Pid::from_raw(pid), signal::Signal::SIGPROF)
            .with_context(|| format!("failed to send SIGPROF to pid {}", pid))
    }
}
