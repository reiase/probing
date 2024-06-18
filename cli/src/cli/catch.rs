use anyhow::Result;
use clap::Args;

/// Handle target process crash
#[derive(Args, Default)]
#[command(version, about, long_about = None)]
pub struct CatchCrashCommand {}

impl CatchCrashCommand {
    pub fn run(&self, _pid: i32) -> Result<()> {
        todo!()
    }
}
