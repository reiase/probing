use anyhow::Result;
use clap::Args;
use probing_common::cli::ProbingCommand;

use crate::cli::usr1_handler;

/// Misc. Commands
#[derive(Args, Default)]
pub struct MiscCommand {
    #[arg(long)]
    show_plt: bool,
}

impl MiscCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        if self.show_plt {
            let cmd = ProbingCommand::ShowPLT;
            let cmd = ron::to_string(&cmd)?;
            return usr1_handler(cmd, pid);
        }

        Err(anyhow::anyhow!("no command specified"))
    }
}
