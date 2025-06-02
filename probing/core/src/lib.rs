pub mod config;
pub mod core;
pub mod trace;

use self::core::Engine;
use self::core::EngineBuilder;

pub fn create_engine() -> EngineBuilder {
    Engine::builder().with_default_namespace("probe")
}

use anyhow::Result;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

pub static ENGINE: Lazy<RwLock<Engine>> = Lazy::new(|| RwLock::new(Engine::default()));

pub async fn initialize_engine(builder: EngineBuilder) -> Result<()> {
    let engine = match builder.build() {
        Ok(engine) => engine,
        Err(e) => {
            log::error!("Error creating engine: {}", e);
            return Err(e.into());
        }
    };

    let mut global_engine = ENGINE.write().await;
    *global_engine = engine;

    Ok(())
}
