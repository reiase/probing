use anyhow::Result;
use clap::Args;
use probing_common::cli::ProbingCommand;

use super::ctrl::CtrlChannel;

/// Performance Diagnosis Tool
#[derive(Args, Default, Debug)]
pub struct PerfCommand {
    /// profiling c/c++ codes
    #[arg(long, conflicts_with_all = ["torch"])]
    cc: bool,

    /// profiling torch models
    #[arg(long, conflicts_with_all = ["cc"])]
    torch: bool,
}

impl PerfCommand {
    pub fn run(&self, ctrl: CtrlChannel) -> Result<()> {
        let cmd = if self.cc {
            ProbingCommand::Perf
        } else if self.torch {
            ProbingCommand::Execute {
                script: "tprofile()".to_string(),
            }
        } else {
            ProbingCommand::Nil
        };
        let cmd = ron::to_string(&cmd)?;
        ctrl.send_ctrl(cmd)
    }
}
