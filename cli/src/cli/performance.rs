use anyhow::{Context, Result};
use clap::Args;
use nix::{sys::signal, unistd::Pid};

/// Start profiling
#[derive(Args, Default)]
pub struct PerfCommand {
    /// profiling c/c++ codes
    #[arg(long, conflicts_with_all = ["torch"])]
    cc: bool,

    /// profiling torch models
    #[arg(long, conflicts_with_all = ["cc"])]
    torch: bool,
}

impl PerfCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        signal::kill(Pid::from_raw(pid), signal::Signal::SIGPROF)
            .with_context(|| format!("failed to send SIGPROF to pid {}", pid))
    }
}
