use core::Engine;
use core::EngineBuilder;

pub mod core;

pub fn create_engine() -> EngineBuilder {
    Engine::builder()
        .with_default_catalog_and_schema("probe", "probe")
        .with_information_schema(true)
}
