use core::Engine;
use std::sync::Arc;

use anyhow::Result;
use plugins::{envs::EnvPlugin, files::FilesPlugin, kmsg::KMsgPlugin};

pub mod core;
pub mod plugins;

fn init_engine(engine: &Engine) -> Result<()> {
    engine.enable("probe", Arc::new(EnvPlugin::new("envs", "process")))?;
    engine.enable("probe", Arc::new(KMsgPlugin::new("kmsg", "system")))?;
    engine.enable("probe", Arc::new(FilesPlugin::default()))?;
    Ok(())
}

pub fn create_engine() -> Engine {
    let engine = Engine::default();

    if let Err(e) = init_engine(&engine) {
        println!("{e}");
    }
    engine
}
