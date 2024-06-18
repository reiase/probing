use anyhow::Result;
use clap::Parser;

mod cli;
mod inject;

pub fn main() -> Result<()> {
    let cli: cli::Cli = cli::Cli::parse();

    cli.run()
}
