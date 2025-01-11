use anyhow::Result;
use env_logger::Env;

mod cli;
mod inject;
mod table;

const ENV_PROBING_LOG: &str = "PROBING_LOG";

pub fn main() -> Result<()> {
    env_logger::init_from_env(Env::new().filter(ENV_PROBING_LOG));
    cli::run()
}
