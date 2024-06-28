use anyhow::Result;

mod cli;
mod inject;

pub fn main() -> Result<()> {
    cli::run()
}
