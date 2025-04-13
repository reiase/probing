pub mod core;

use self::core::Engine;
use self::core::EngineBuilder;

pub fn create_engine() -> EngineBuilder {
    Engine::builder().with_default_namespace("probe")
}
