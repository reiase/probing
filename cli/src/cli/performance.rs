use anyhow::{Context, Result};
use clap::Args;
use nix::{sys::signal, unistd::Pid};
use probing_common::cli::ProbingCommand;

use super::send_ctrl;

/// Performance Diagnosis Tool
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
        if self.cc {
            return signal::kill(Pid::from_raw(pid), signal::Signal::SIGPROF)
                .with_context(|| format!("failed to send SIGPROF to pid {}", pid));
        }
        if self.torch {
            let cmd = ProbingCommand::Execute {
                script: "tprofile()".to_string(),
            };
            let cmd = ron::to_string(&cmd)?;
            return send_ctrl(cmd, pid);
        }
        Ok(())
    }
}
