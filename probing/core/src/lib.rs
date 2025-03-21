use core::Engine;
use core::EngineBuilder;
use std::sync::Arc;

use plugins::{envs::EnvPlugin, file::FilePlugin, files::FilesPlugin};

pub mod core;
pub mod plugins;

#[cfg(feature = "kmsg")]
fn enable_kmsg(builder: EngineBuilder) -> EngineBuilder {
    use plugins::kmsg::KMsgPlugin;
    builder.with_plugin("probe", Arc::new(KMsgPlugin::new("kmsg", "system")))
}

#[cfg(not(feature = "kmsg"))]
fn enable_kmsg(builder: EngineBuilder) -> EngineBuilder {
    builder
}

pub fn create_engine() -> EngineBuilder {
    let mut builder = Engine::builder()
        .with_default_catalog_and_schema("probe", "probe")
        .with_information_schema(true);

    builder = builder.with_plugin("probe", Arc::new(EnvPlugin::new("envs", "process")));
    builder = builder.with_plugin("probe", Arc::new(FilePlugin::new("file")));
    builder = builder.with_plugin("probe", Arc::new(FilesPlugin::default()));
    builder = enable_kmsg(builder);

    builder
}
