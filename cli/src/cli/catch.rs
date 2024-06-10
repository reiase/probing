use anyhow::Result;
use argh::FromArgs;

/// Handle target process crash
#[derive(FromArgs)]
#[argh(subcommand, name = "catch")]
pub struct CatchCrashCommand {}

impl CatchCrashCommand {
    pub fn run(&self, _pid: i32) -> Result<()> {
        todo!()
    }
}
