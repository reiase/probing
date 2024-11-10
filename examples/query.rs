use std::sync::Arc;

use anyhow::Result;

use engine::core::pretty;
use engine::core::Engine;
use engine::plugins::env::EnvPlugin;
use engine::plugins::files::FilesPlugin;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::new();

    engine.enable("probe".to_string(), Arc::new(EnvPlugin::default()))?;
    engine.enable("probe".to_string(), Arc::new(FilesPlugin::default()))?;

    let query = std::env::args().collect::<Vec<_>>()[1].clone();
    let rb = engine.sql(query.as_str()).await?.collect().await?;

    pretty::print_batches(&rb)?;

    Ok(())
}
