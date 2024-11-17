use core::Engine;
use std::sync::Arc;

use anyhow::Result;
use plugins::{env::EnvPlugin, env2::EnvPlugin2, files::FilesPlugin, kmsg::KMsgPlugin};

pub mod core;
pub mod plugins;

fn init_engine(engine: &Engine) ->Result<()> {
    engine.enable("probe".to_string(), Arc::new(EnvPlugin::default()))?;
    engine.enable("probe".to_string(), Arc::new(FilesPlugin::default()))?;
    engine.enable("probe".to_string(), Arc::new(EnvPlugin2::new("envs2".to_string(), "process".to_string())))?;
    engine.enable("probe".to_string(), Arc::new(KMsgPlugin::new("kmsg".to_string(), "system".to_string())))?;
    Ok(())
}

pub fn create_engine() -> Engine {
    let engine = Engine::new();

    if let Err(e) = init_engine(&engine) {
        println!("{e}");
    }
    engine
}