use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use arrow::ipc::writer::{IpcWriteOptions, StreamWriter};
use arrow::ipc::MetadataVersion;
use datafusion::arrow::array::RecordBatch;
use datafusion::catalog::{CatalogProvider, SchemaProvider};
use datafusion::catalog_common::{MemoryCatalogProvider, MemorySchemaProvider};
use datafusion::error::Result;
use datafusion::execution::SessionState;
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};
use futures;

#[derive(PartialEq, Eq)]
pub enum PluginType {
    TableProviderPlugin,
    SchemaProviderPlugin,
}
pub trait Plugin {
    fn name(&self) -> String;
    fn kind(&self) -> PluginType;
    fn category(&self) -> String;

    #[allow(unused)]
    fn register_table(&self, schema: Arc<dyn SchemaProvider>, state: &SessionState) -> Result<()> {
        Ok(())
    }

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
    context: SessionContext,
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl Engine {
    pub fn new() -> Self {
        let config = SessionConfig::default()
            .with_information_schema(true)
            .with_default_catalog_and_schema("probe", "probe");
        let engine = Engine {
            context: SessionContext::new_with_config(config),
            plugins: Default::default(),
        };

        engine
    }

    pub fn builder() -> EngineBuilder {
        EngineBuilder::new()
    }

    pub async fn sql(self, query: &str) -> Result<DataFrame> {
        self.context.sql(query).await
    }

    pub async fn async_execute(self, query: String) -> Result<Vec<RecordBatch>> {
        self.context.sql(query.as_str()).await?.collect().await
    }

    pub fn execute(self, query: &str) -> anyhow::Result<Vec<u8>> {
        futures::executor::block_on(async {
            let res = self.sql(query).await?.collect().await?;
            let buffer: Vec<u8> = Vec::new();
            let schema = res[0].schema();
            let options = IpcWriteOptions::try_new(8, false, MetadataVersion::V5)?;
            let mut writer = StreamWriter::try_new_with_options(buffer, &schema, options)?;
            for batch in res.iter() {
                writer.write(batch)?;
            }
            writer.finish()?;
            Ok(writer.into_inner()?)
        })
    }

    pub fn enable<S: AsRef<str>>(&self, domain: S, plugin: Arc<dyn Plugin>) -> Result<()> {
        let category = plugin.category();

        let catalog = if let Some(catalog) = self.context.catalog(domain.as_ref()) {
            catalog
        } else {
            self.context
                .register_catalog(category.clone(), Arc::new(MemoryCatalogProvider::new()));
            self.context.catalog(domain.as_ref()).unwrap()
        };

        if plugin.kind() == PluginType::SchemaProviderPlugin {
            let state: SessionState = self.context.state();
            plugin.register_schema(catalog, &state)?;
            if let Ok(mut maps) = self.plugins.write() {
                maps.insert(format!("{}.{}", domain.as_ref(), category), plugin);
            }
        } else if plugin.kind() == PluginType::TableProviderPlugin {
            let schema = if catalog.schema_names().contains(&category) {
                catalog.schema(&category.as_str())
            } else {
                let schema = MemorySchemaProvider::new();
                catalog.register_schema(category.as_str(), Arc::new(schema))?;
                catalog.schema(&category.as_str())
            };
            let state: SessionState = self.context.state();
            plugin.register_table(schema.unwrap(), &state)?;
            if let Ok(mut maps) = self.plugins.write() {
                maps.insert(
                    format!("{}.{}.{}", domain.as_ref(), category, plugin.name()),
                    plugin,
                );
            }
        }
        Ok(())
    }
}

// Define the EngineBuilder struct
pub struct EngineBuilder {
    config: SessionConfig,
    plugins: Vec<(String, Arc<dyn Plugin>)>,
}

impl EngineBuilder {
    // Create a new EngineBuilder with default settings
    pub fn new() -> Self {
        EngineBuilder {
            config: SessionConfig::default(),
            plugins: Vec::new(),
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
    pub fn with_plugin(mut self, namespace: String, plugin: Arc<dyn Plugin>) -> Self {
        self.plugins.push((namespace, plugin));
        self
    }

    // Build the Engine with the specified configurations
    pub fn build(self) -> Result<Engine> {
        let context = SessionContext::new_with_config(self.config);
        let engine = Engine {
            context,
            plugins: Default::default(),
        };
        for (namespace, plugin) in self.plugins {
            engine.enable(namespace.as_str(), plugin)?;
        }
        Ok(engine)
    }
}
