mod chunked_encode;
mod engine;
mod error;
mod extension;
mod table_plugin;

pub use engine::Engine;
pub use engine::EngineBuilder;
pub use engine::Plugin;
pub use engine::PluginType;

pub use table_plugin::CustomSchema;
pub use table_plugin::CustomTable;
pub use table_plugin::SchemaPlugin;
pub use table_plugin::TablePlugin;

pub use table_plugin::LazyTableSource;

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
pub use datafusion::arrow::util::pretty;
pub use datafusion::common::error::DataFusionError;
pub use datafusion::config::CatalogOptions;
pub use datafusion::config::ConfigEntry;
pub use datafusion::config::ConfigExtension;
pub use datafusion::config::ExtensionOptions;

// pub static ENGINE_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
//     tokio::runtime::Builder::new_multi_thread()
//         .worker_threads(4)
//         .enable_all()
//         .build()
//         .unwrap()
// });

#[cfg(test)]
mod specs {
    use super::*;
    use rspec;

    #[test]
    fn engine_specs() {
        rspec::run(&rspec::describe(
            "Build `Engine` with `EngineBuilder`",
            (),
            |ctx| {
                ctx.specify("EngineBuilder supports different options", |ctx| {
                    ctx.it("build engine with information schema", |_| {
                        let engine = Engine::builder().with_information_schema(true).build();
                        assert!(engine.is_ok());

                        let show_tables = engine.unwrap().query("show tables");
                        assert!(show_tables.is_ok());
                    });

                    ctx.it("build engine with default catalog and schema", |_| {
                        let engine = Engine::builder()
                            .with_default_catalog_and_schema("probe", "probe")
                            .build();
                        assert!(engine.is_ok());
                    });
                });

                ctx.specify("Execute querues with `Engine`", |ctx| {
                    ctx.it("execute `show tables`", |_| {
                        let engine = Engine::builder()
                            .with_information_schema(true)
                            .build()
                            .unwrap();

                        let show_tables = engine.query("show tables");
                        assert!(show_tables.is_ok());
                    });

                    ctx.it("execute `SELECT 1 as val`", move |_| {
                        let engine = Engine::builder()
                            .with_information_schema(true)
                            .build()
                            .unwrap();

                        let show_schemas = engine.query("SELECT 1 as val");
                        assert!(show_schemas.is_ok());
                    });
                });
            },
        ));
    }
}
