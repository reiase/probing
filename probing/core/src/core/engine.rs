use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use arrow::array::Float32Array;
use arrow::array::Float64Array;
use arrow::array::Int32Array;
use arrow::array::Int64Array;
use arrow::array::StringArray;
use arrow::array::TimestampMicrosecondArray;
use arrow::compute::concat_batches;
use datafusion::catalog::MemoryCatalogProvider;
use datafusion::catalog::MemorySchemaProvider;
use datafusion::catalog::{CatalogProvider, SchemaProvider};
use datafusion::config::ConfigExtension;
use datafusion::error::DataFusionError;
use datafusion::error::Result;
use datafusion::execution::SessionState;
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};
use futures;

use super::extension::EngineExtension;
use super::extension::EngineExtensionManager;
use probing_proto::types::Seq;

/// Defines the types of plugins supported by the Probing query engine.
/// These plugin types determine how data sources are registered with the engine.
#[derive(PartialEq, Eq)]
pub enum PluginType {
    /// Provides a single table with fixed structure.
    /// Suitable for hardware metrics, process stats, and performance counter data.
    /// Tables are accessible via SQL as "category.name".
    Table,

    /// Provides an entire schema (collection of tables).
    /// Suitable for file system monitoring, Python module tracking, or dynamically
    /// generated performance data.
    /// Tables in a schema are accessible via SQL as "category.table_name".
    Schema,
}

/// Low-level interface for extending engine functionality through plugins
///
/// Plugins can register either schemas (collections of tables) or
/// individual tables to the query engine. Implementations should
/// handle specific data sources or analysis capabilities.
///
/// # Naming Convention
///
/// Data in the engine is organized hierarchically:
///
/// - Catalog (default is "probe")
///   - Schema (provided by plugin's "category")
///     - Table (provided by plugin's "name" or dynamically by SchemaProvider)
///
/// ## For Table Plugins
///
/// A table plugin must provide both a category and a name. The table will be
/// accessible in SQL queries as:
///
/// ```sql
/// SELECT * FROM probe.category.name
/// ```
///
/// ## For Schema Plugins
///
/// A schema plugin only needs to provide a category. The tables within the schema
/// will be accessible in SQL queries as:
///
/// ```sql
/// SELECT * FROM probe.category.some_table_name
/// ```
///
/// where `some_table_name` is any table provided by the schema plugin.
pub trait Plugin {
    /// Returns the unique name of the plugin.
    ///
    /// For Table plugins, this is the table name.
    /// For Schema plugins, this is the schema name.
    fn name(&self) -> String;

    /// Returns the type of this plugin, determining how it integrates with the engine.
    /// This controls which registration method will be called (register_table or register_schema).
    fn kind(&self) -> PluginType;

    /// Returns the category for this plugin, used for organizing related tables.
    ///
    /// - For Table plugins, this defines the schema name under which the table is registered.
    ///   The table will be accessible as "category.name".
    ///
    /// - For Schema plugins, this defines the name of the schema being provided.
    ///   Tables in this schema will be accessible as "category.table_name".
    fn category(&self) -> String;

    /// Registers a table with the provided schema.
    ///
    /// Implemented by Table plugins to register their data source
    /// with the query engine. The default implementation does nothing.
    ///
    /// # Arguments
    /// * `schema` - The schema provider to register the table with
    /// * `state` - The current session state
    #[allow(unused)]
    fn register_table(&self, schema: Arc<dyn SchemaProvider>, state: &SessionState) -> Result<()> {
        Ok(())
    }

    /// Registers a schema with the provided catalog.
    ///
    /// Implemented by Schema plugins to register their schema
    /// with the query engine. The default implementation does nothing.
    ///
    /// # Arguments
    /// * `catalog` - The catalog provider to register the schema with
    /// * `state` - The current session state
    #[allow(unused)]
    fn register_schema(
        &self,
        catalog: Arc<dyn CatalogProvider>,
        state: &SessionState,
    ) -> Result<()> {
        Ok(())
    }
}

/// Core query engine for the Probing system
///
/// The Engine provides SQL query capabilities over various data sources
/// through a plugin system. It wraps DataFusion's SessionContext and manages
/// the lifecycle of registered plugins.
///
/// # Data Organization
///
/// Data in the engine is organized hierarchically:
/// - Catalog (default is "probe")
///   - Schema (provided by plugins' "category")
///     - Table (provided by plugins)
///
/// # Usage Example
///
/// ```
/// // Create a new engine using the builder pattern
/// let engine = probing_core::core::Engine::builder()
///     .with_information_schema(true)
///     .with_default_catalog_and_schema("probe", "example_schema")
///     .build().unwrap();
///
/// // Execute a SQL query
/// let result = engine.query("SELECT * FROM information_schema.tables");
/// ```
pub struct Engine {
    /// DataFusion session context for executing SQL queries
    pub context: SessionContext,
    /// Registry of enabled plugins, mapped by their fully qualified names
    plugins: RwLock<HashMap<String, Arc<dyn Plugin + Sync + Send>>>,
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            plugins: RwLock::new(self.plugins.read().unwrap().clone()),
        }
    }
}

impl Default for Engine {
    /// Creates a new Engine instance with default configuration
    ///
    /// The default engine:
    /// - Enables the information schema for metadata queries
    /// - Sets "probe" as both the default catalog and schema
    /// - Has no plugins registered initially
    fn default() -> Self {
        let config = SessionConfig::default()
            .with_information_schema(true)
            .with_default_catalog_and_schema("probe", "probe");
        Engine {
            context: SessionContext::new_with_config(config),
            plugins: Default::default(),
        }
    }
}

impl Engine {
    pub fn builder() -> EngineBuilder {
        EngineBuilder::new()
    }

    pub fn register_extension_options<T: ConfigExtension>(&self, extension: T) {
        self.context
            .state()
            .config_mut()
            .options_mut()
            .extensions
            .insert(extension);
    }

    pub async fn sql(&self, query: &str) -> Result<DataFrame> {
        self.context.sql(query).await
    }

    pub async fn async_query<T: Into<String>>(
        &self,
        query: T,
    ) -> Result<probing_proto::prelude::DataFrame> {
        let query: String = query.into();
        let batches = self.sql(query.as_str()).await?.collect().await?;
        if batches.is_empty() {
            return Ok(probing_proto::prelude::DataFrame::default());
        }
        let batch = concat_batches(&batches[0].schema(), batches.iter())?;

        let names = batch
            .schema()
            .fields()
            .iter()
            .map(|x| x.name().clone())
            .collect::<Vec<_>>();
        let columns = batch
            .columns()
            .iter()
            .map(|col| {
                if let Some(array) = col.as_any().downcast_ref::<Int32Array>() {
                    Seq::SeqI32(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<Int64Array>() {
                    Seq::SeqI64(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<Float32Array>() {
                    Seq::SeqF32(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<Float64Array>() {
                    Seq::SeqF64(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<StringArray>() {
                    Seq::SeqText((0..col.len()).map(|x| array.value(x).to_string()).collect())
                } else if let Some(array) = col.as_any().downcast_ref::<TimestampMicrosecondArray>()
                {
                    Seq::SeqI64(array.values().to_vec())
                } else {
                    Seq::Nil
                }
            })
            .collect::<Vec<_>>();
        Ok(probing_proto::prelude::DataFrame::new(names, columns))
    }

    pub fn query<T: Into<String>>(&self, q: T) -> Result<probing_proto::prelude::DataFrame> {
        futures::executor::block_on(async { self.async_query(q).await })
    }

    /// Get default schema from configuration
    pub fn default_schema(&self) -> String {
        self.context
            .state()
            .config()
            .options()
            .catalog
            .default_schema
            .clone()
    }

    pub fn enable(&self, plugin: Arc<dyn Plugin + Sync + Send>) -> Result<()> {
        let category = plugin.category();

        let catalog = if let Some(catalog) = self.context.catalog("probe") {
            catalog
        } else {
            self.context
                .register_catalog("probe", Arc::new(MemoryCatalogProvider::new()));
            self.context
                .catalog("probe")
                .ok_or_else(|| DataFusionError::Internal("no catalog `probe`".to_string()))?
        };

        if plugin.kind() == PluginType::Schema {
            let state: SessionState = self.context.state();
            plugin.register_schema(catalog, &state)?;
            if let Ok(mut maps) = self.plugins.write() {
                maps.insert(format!("probe.{}", category), plugin);
            }
        } else if plugin.kind() == PluginType::Table {
            let schema = if catalog.schema_names().contains(&category) {
                catalog.schema(category.as_str())
            } else {
                let schema = MemorySchemaProvider::new();
                catalog.register_schema(category.as_str(), Arc::new(schema))?;
                catalog.schema(category.as_str())
            }
            .ok_or_else(|| DataFusionError::Internal(format!("schema `{}` not found", category)))?;
            let state: SessionState = self.context.state();
            plugin.register_table(schema, &state)?;
            if let Ok(mut maps) = self.plugins.write() {
                maps.insert(format!("probe.{}.{}", category, plugin.name()), plugin);
            }
        }
        Ok(())
    }
}

// Define the EngineBuilder struct
pub struct EngineBuilder {
    config: SessionConfig,
    plugins: Vec<Arc<dyn Plugin + Sync + Send>>,
    extensions: Vec<Arc<Mutex<dyn EngineExtension + Send + Sync>>>,
}

impl EngineBuilder {
    // Create a new EngineBuilder with default settings
    pub fn new() -> Self {
        EngineBuilder {
            config: SessionConfig::default(),
            plugins: Vec::new(),
            extensions: Vec::new(),
        }
    }

    // Set the default catalog and schema
    pub fn with_default_catalog_and_schema(mut self, catalog: &str, schema: &str) -> Self {
        self.config = self.config.with_default_catalog_and_schema(catalog, schema);
        self
    }

    // Enable or disable the information schema
    pub fn with_information_schema(mut self, enabled: bool) -> Self {
        self.config = self.config.with_information_schema(enabled);
        self
    }

    // Add a plugin to the builder
    pub fn with_plugin(mut self, plugin: Arc<dyn Plugin + Sync + Send>) -> Self {
        self.plugins.push(plugin);
        self
    }

    pub fn with_extension<T>(mut self, ext: T, category: &str, name: Option<&str>) -> Self
    where
        T: EngineExtension + Send + Sync + 'static,
    {
        let ext = Arc::new(Mutex::new(ext));
        if let Some(datasrc) = ext.lock().unwrap().datasrc(category, name) {
            self.plugins.push(datasrc)
        };
        self.extensions.push(ext);
        self
    }

    // Build the Engine with the specified configurations
    pub fn build(mut self) -> Result<Engine> {
        let mut eem = EngineExtensionManager::default();
        for extension in self.extensions {
            eem.register(extension);
        }
        self.config.options_mut().extensions.insert(eem);

        let context = SessionContext::new_with_config(self.config);
        let engine = Engine {
            context,
            plugins: Default::default(),
        };
        for plugin in self.plugins {
            engine.enable(plugin)?;
        }

        Ok(engine)
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
    use arrow::record_batch::RecordBatch;
    use datafusion::datasource::TableProvider;
    use datafusion::execution::context::SessionState;
    use datafusion::logical_expr::{Expr, TableType};
    use datafusion::physical_plan::ExecutionPlan;
    use std::any::Any;
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct TestTablePlugin {
        schema: SchemaRef,
        batches: Vec<RecordBatch>,
    }

    impl Default for TestTablePlugin {
        fn default() -> Self {
            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int32, false),
                Field::new("name", DataType::Utf8, false),
            ]));

            // create data
            let id_array = Int32Array::from(vec![1, 2, 3]);
            let name_array = StringArray::from(vec!["a", "b", "c"]);

            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(id_array), Arc::new(name_array)],
            )
            .unwrap();
            Self {
                schema,
                batches: vec![batch],
            }
        }
    }

    impl Plugin for TestTablePlugin {
        fn name(&self) -> String {
            "test_table".to_string()
        }

        fn kind(&self) -> PluginType {
            PluginType::Table
        }

        fn category(&self) -> String {
            "test_schema".to_string()
        }

        fn register_table(
            &self,
            schema_provider: Arc<dyn SchemaProvider>,
            _state: &SessionState,
        ) -> Result<()> {
            schema_provider.register_table(self.name(), Arc::new(self.clone()))?;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl TableProvider for TestTablePlugin {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn schema(&self) -> SchemaRef {
            self.schema.clone()
        }

        fn table_type(&self) -> TableType {
            TableType::Base
        }

        async fn scan(
            &self,
            _ctx: &dyn datafusion::catalog::Session,
            projection: Option<&Vec<usize>>,
            _filters: &[Expr],
            _limit: Option<usize>,
        ) -> Result<Arc<dyn ExecutionPlan>> {
            Ok(Arc::new(
                datafusion::physical_plan::memory::MemoryExec::try_new(
                    &[self.batches.clone()],
                    self.schema.clone(),
                    projection.cloned(),
                )?,
            ))
        }
    }

    #[derive(Default)]
    struct TestSchemaPlugin {}

    impl Plugin for TestSchemaPlugin {
        fn name(&self) -> String {
            "test_schema".to_string()
        }

        fn kind(&self) -> PluginType {
            PluginType::Schema
        }

        fn category(&self) -> String {
            "test_schema".to_string()
        }
    }

    #[tokio::test]
    async fn test_engine_builder() {
        // testing default builder
        let engine = Engine::builder().build().unwrap();
        assert_eq!(engine.default_schema(), "public"); // 'public' is the default schema in DataFusion

        // building with custom catalog and schema
        let engine = Engine::builder()
            .with_default_catalog_and_schema("test_catalog", "test_schema")
            .build()
            .unwrap();
        assert_eq!(engine.default_schema(), "test_schema");
    }

    #[tokio::test]
    async fn test_plugin_with_data() -> Result<()> {
        // create engine
        let engine = Engine::builder().with_information_schema(true).build()?;

        // register table plugin
        let plugin = Arc::new(TestTablePlugin::default());
        engine.enable(plugin)?;

        // verify table registration
        let result = engine.query("SELECT * FROM probe.test_schema.test_table")?;

        assert_eq!(result.names.len(), 2);
        assert_eq!(result.names[0], "id");
        assert_eq!(result.names[1], "name");

        // verify data
        let result = engine.query("SELECT * FROM probe.test_schema.test_table WHERE id > 1")?;
        if let Seq::SeqI32(ids) = &result.cols[0] {
            assert_eq!(ids.len(), 2); // expect 2 rows
            assert!(ids.iter().all(|&id| id > 1)); // with id > 1
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_extension_registration() {
        #[derive(Debug)]
        struct TestExtension;

        impl EngineExtension for TestExtension {
            fn name(&self) -> String {
                "test_extension".to_string()
            }

            fn datasrc(&self, _: &str, _: Option<&str>) -> Option<Arc<dyn Plugin + Send + Sync>> {
                Some(Arc::new(TestTablePlugin::default()))
            }
        }

        // register extension
        let engine = Engine::builder()
            .with_default_catalog_and_schema("probe", "probe")
            .with_information_schema(true)
            .with_extension(TestExtension, "test_schema", Some("test_table"))
            .build()
            .unwrap();

        // 验证插件是否正确注册
        let result = engine.query("SELECT * FROM test_schema.test_table");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let engine = Engine::builder().build().unwrap();

        // 测试Table插件注册
        let table_plugin = Arc::new(TestTablePlugin::default());
        assert!(engine.enable(table_plugin).is_ok());

        // 测试Schema插件注册
        let schema_plugin = Arc::new(TestSchemaPlugin::default());
        assert!(engine.enable(schema_plugin).is_ok());
    }

    #[tokio::test]
    async fn test_basic_queries() {
        let engine = Engine::builder()
            .with_information_schema(true)
            .build()
            .unwrap();

        // testing basic SELECT query
        let result = engine.query("SELECT 1 as num, 'test' as str").unwrap();
        assert_eq!(result.names.len(), 2);
        assert_eq!(result.names[0], "num");
        assert_eq!(result.names[1], "str");

        // testing empty result set
        let result = engine.query("SELECT 1 WHERE 1=0").unwrap();
        assert!(result.names.is_empty());
    }

    #[tokio::test]
    async fn test_query_error_handling() {
        let engine = Engine::builder().build().unwrap();

        // testing invalid SQL syntax
        let result = engine.query("SELECT invalid syntax");
        assert!(result.is_err());

        // testing nonexistent table
        let result = engine.query("SELECT * FROM nonexistent_table");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_queries() {
        use futures::future::join_all;

        let engine = Engine::builder().build().unwrap();
        let queries = ["SELECT 1", "SELECT 2", "SELECT 3"];

        let handles: Vec<_> = queries
            .iter()
            .map(|q| {
                let engine = engine.clone();
                let query = q.to_string();
                tokio::spawn(async move { engine.async_query(query).await })
            })
            .collect();

        let results = join_all(handles).await;
        for result in results {
            assert!(result.unwrap().is_ok());
        }
    }

    #[tokio::test]
    async fn test_data_types() {
        let engine = Engine::builder().build().unwrap();

        let query = "
            SELECT 
                CAST(1 AS INT) as int_val,
                CAST(2.5 AS FLOAT) as float_val,
                'test' as string_val
        ";

        let result = engine.query(query).unwrap();
        assert_eq!(result.names.len(), 3);

        // testing data types
        assert!(matches!(result.cols[0], Seq::SeqI32(_)));
        assert!(matches!(result.cols[1], Seq::SeqF32(_)));
        assert!(matches!(result.cols[2], Seq::SeqText(_)));
    }

    #[test]
    fn test_engine_builder_configuration() {
        let builder = Engine::builder()
            .with_default_catalog_and_schema("test_catalog", "test_schema")
            .with_information_schema(true);

        // testing default catalog and schema
        let engine = builder.build().unwrap();
        assert_eq!(engine.default_schema(), "test_schema");

        // testing information schema
        let result = engine.query("SHOW TABLES");
        assert!(result.is_ok());
    }
}
