use std::sync::Arc;

use anyhow::Result;

// use engine::core::pretty;
use engine::core::Engine;
use engine::plugins::envs::EnvPlugin;
use engine::plugins::files::FilesPlugin;
use engine::plugins::kmsg::KMsgPlugin;

use cli::table::render_table;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::new();

    engine.enable("probe", Arc::new(FilesPlugin::default()))?;
    engine.enable("probe", Arc::new(EnvPlugin::new("envs", "process")))?;
    engine.enable("probe", Arc::new(KMsgPlugin::new("kmsg", "system")))?;

    let query = std::env::args().collect::<Vec<_>>()[1].clone();
    let rb = engine.sql(query.as_str()).await?.collect().await?;

    // pretty::print_batches(&rb)?;
    render_table(rb.as_slice());

    Ok(())
}
