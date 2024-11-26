use anyhow::Result;

mod cli;
mod inject;
mod table;

pub fn main() -> Result<()> {
    cli::run()
}
