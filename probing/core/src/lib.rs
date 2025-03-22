use core::Engine;
use core::EngineBuilder;
use std::sync::Arc;

use plugins::{envs::EnvPlugin, file::FilePlugin, files::FilesPlugin};

pub mod core;
pub mod plugins;

pub fn create_engine() -> EngineBuilder {
    let mut builder = Engine::builder()
        .with_default_catalog_and_schema("probe", "probe")
        .with_information_schema(true);

    builder = builder.with_plugin(EnvPlugin::create("process", "envs"));
    builder = builder.with_plugin(FilePlugin::create("file"));
    builder = builder.with_plugin(Arc::new(FilesPlugin::default()));

    builder
}
