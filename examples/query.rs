use std::sync::Arc;

use anyhow::Result;

use probing_engine::core::Engine;
use probing_engine::plugins::envs::EnvPlugin;
use probing_engine::plugins::files::FilesPlugin;
use probing_engine::plugins::file::FilePlugin;
use probing_engine::plugins::kmsg::KMsgPlugin;

use cli::table::render_table;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::new();

    engine.enable("probe", Arc::new(FilesPlugin::default()))?;
    engine.enable("probe", Arc::new(FilePlugin::new("file")))?;
    engine.enable("probe", Arc::new(EnvPlugin::new("envs", "process")))?;
    engine.enable("probe", Arc::new(KMsgPlugin::new("kmsg", "system")))?;

    let query = std::env::args().collect::<Vec<_>>()[1].clone();
    let rb = engine.sql(query.as_str()).await?.collect().await?;

    render_table(rb.as_slice());

    Ok(())
}
