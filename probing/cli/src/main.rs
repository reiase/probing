use anyhow::Result;
use clap::Parser;
use env_logger::Env;

mod cli;
mod inject;
mod table;

const ENV_PROBING_LOGLEVEL: &str = "PROBING_LOGLEVEL";

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init_from_env(Env::new().filter(ENV_PROBING_LOGLEVEL));
    cli::Cli::parse().run().await
}
