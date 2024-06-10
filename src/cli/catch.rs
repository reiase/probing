use argh::FromArgs;
use anyhow::Result;

/// Handle target process crash
#[derive(FromArgs)]
#[argh(subcommand, name = "catch")]
pub struct CatchCrashCommand {}

impl CatchCrashCommand {
    pub fn run(&self, pid: i32) -> Result<()> {
        todo!()
    }
}
