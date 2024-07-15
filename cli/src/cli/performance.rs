use anyhow::Result;
use clap::Args;
use probing_common::cli::CtrlSignal;

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
            CtrlSignal::Perf
        } else if self.torch {
            CtrlSignal::Execute {
                script: "tprofile()".to_string(),
            }
        } else {
            CtrlSignal::Nil
        };
        let cmd = ron::to_string(&cmd)?;
        ctrl.send_ctrl(cmd)
    }
}
