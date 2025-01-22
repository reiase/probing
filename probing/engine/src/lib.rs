use core::Engine;
use core::EngineBuilder;
use std::sync::Arc;

use plugins::{envs::EnvPlugin, file::FilePlugin, files::FilesPlugin, kmsg::KMsgPlugin};

pub mod core;
pub mod plugins;

pub fn create_engine() -> EngineBuilder {
    Engine::builder()
        .with_default_catalog_and_schema("probe", "probe")
        .with_information_schema(true)
        .with_plugin("probe", Arc::new(EnvPlugin::new("envs", "process")))
        .with_plugin("probe", Arc::new(KMsgPlugin::new("kmsg", "system")))
        .with_plugin("probe", Arc::new(FilePlugin::new("file")))
        .with_plugin("probe", Arc::new(FilesPlugin::default()))
}
