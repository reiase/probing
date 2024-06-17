use anyhow::Result;

mod cli;
mod inject;

pub fn main() -> Result<()> {
    let cli: cli::Cli = argh::from_env();

    cli.run()
}
