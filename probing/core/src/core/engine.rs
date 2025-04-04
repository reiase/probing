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

pub struct Engine {
    pub context: SessionContext,
    plugins: RwLock<HashMap<String, Arc<dyn Plugin + Sync + Send>>>,
}

impl Default for Engine {
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

    fn with_extension_class<T>(mut self) -> Self
    where
        T: EngineExtension + Send + Sync + Default + 'static,
    {
        let ext = Arc::new(Mutex::new(T::default()));
        self.extensions.push(ext);
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
