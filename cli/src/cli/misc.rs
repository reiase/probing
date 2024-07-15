use anyhow::Result;
use clap::Args;
use probing_common::cli::CtrlSignal;

use super::ctrl::CtrlChannel;

/// Misc. Commands
#[derive(Args, Default, Debug)]
pub struct MiscCommand {
    #[arg(long)]
    show_plt: bool,
}

impl MiscCommand {
    pub fn run(&self, ctrl: CtrlChannel) -> Result<()> {
        if self.show_plt {
            let cmd = CtrlSignal::ShowPLT;
            let cmd = ron::to_string(&cmd)?;
            return ctrl.send_ctrl(cmd);
        }
        Err(anyhow::anyhow!("no command specified"))
    }
}
