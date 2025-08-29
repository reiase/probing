pub mod cluster;
pub mod cluster_model;
mod engine;
mod error;
pub mod extension;
mod plugin;

pub use engine::Engine;
pub use engine::EngineBuilder;
pub use engine::Plugin;
pub use engine::PluginType;

pub use error::EngineError;
pub use error::Result;

pub use plugin::CustomNamespace;
pub use plugin::CustomTable;
pub use plugin::LazyTableSource;
pub use plugin::NamespacePluginHelper;
pub use plugin::TablePluginHelper;

pub use extension::EngineCall;
pub use extension::EngineDatasource;
pub use extension::EngineExtension;
pub use extension::EngineExtensionManager;
pub use extension::EngineExtensionOption;
pub use extension::Maybe;

pub use probing_macros::EngineExtension;

pub use datafusion::arrow::array::ArrayRef;
pub use datafusion::arrow::array::Float32Array;
pub use datafusion::arrow::array::Float64Array;
pub use datafusion::arrow::array::Int32Array;
pub use datafusion::arrow::array::Int64Array;
pub use datafusion::arrow::array::RecordBatch;
pub use datafusion::arrow::array::StringArray;
pub use datafusion::arrow::datatypes::DataType;
pub use datafusion::arrow::datatypes::Field;
pub use datafusion::arrow::datatypes::Schema;
pub use datafusion::arrow::datatypes::SchemaRef;
pub use datafusion::arrow::datatypes::TimeUnit;
pub use datafusion::arrow::util::pretty;
pub use datafusion::common::error::DataFusionError;
pub use datafusion::config::CatalogOptions;

// pub static ENGINE_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
//     tokio::runtime::Builder::new_multi_thread()
//         .worker_threads(4)
//         .enable_all()
//         .build()
//         .unwrap()
// });

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn build_engine() {
        let engine = Engine::builder().build().unwrap();

        let result = engine.async_query("show tables").await;
        assert!(result.is_ok(), "Should execute SHOW TABLES query");
    }

    #[tokio::test]
    async fn build_engine_with_default_namespace() {
        let engine = Engine::builder()
            .with_default_namespace("test")
            .build()
            .unwrap();

        assert_eq!(engine.default_namespace(), "test".to_string());
    }

    #[tokio::test]
    async fn execute_basic_queries() {
        let engine = Engine::builder().build().unwrap();

        // Test SHOW TABLES
        let show_tables = engine.async_query("show tables").await;
        assert!(show_tables.is_ok(), "SHOW TABLES should succeed");

        // Test SELECT
        let select_query = engine.async_query("SELECT 1 as val").await;
        assert!(select_query.is_ok(), "SELECT should return results");

        // Verify result schema
        let df = select_query.unwrap();
        assert_eq!(df.names.len(), 1, "Should have one column");
        assert_eq!(df.names[0], "val", "Column name should match");
        assert!(!df.cols.is_empty(), "Should have data columns");
    }
}
