use anyhow::{Context, Result};
use clap::{Args, ValueEnum};

use nix::{sys::signal, unistd::Pid};

#[derive(Default, ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum PerfTarget {
    #[default]
    cc,
    torch,
}

/// Start profiling
#[derive(Args, Default)]
#[command(version, about, long_about = None)]
pub struct PerfCommand {
    /// performance profiling target
    #[arg(value_enum, default_value_t=PerfTarget::cc)]
    target: PerfTarget,
}

impl PerfCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        signal::kill(Pid::from_raw(pid), signal::Signal::SIGPROF)
            .with_context(|| format!("failed to send SIGPROF to pid {}", pid))
    }
}
