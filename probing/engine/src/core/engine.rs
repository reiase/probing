use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use arrow::array::Float32Array;
use arrow::array::Float64Array;
use arrow::array::Int32Array;
use arrow::array::Int64Array;
use arrow::array::StringArray;
use arrow::array::TimestampMicrosecondArray;
use arrow::compute::concat_batches;
use datafusion::catalog::{CatalogProvider, SchemaProvider};
use datafusion::catalog_common::{MemoryCatalogProvider, MemorySchemaProvider};
use datafusion::error::Result;
use datafusion::execution::SessionState;
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};
use futures;

use super::chunked_encode::chunked_encode;
use probing_proto::types::Array;

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

    pub async fn sql(&self, query: &str) -> Result<DataFrame> {
        self.context.sql(query).await
    }

    pub fn query<T: Into<String>>(&self, q: T) -> anyhow::Result<probing_proto::prelude::DataFrame> {
        let batch = futures::executor::block_on(async {
            let q: String = q.into();
            let batches = self.sql(q.as_str()).await?.collect().await?;
            if batches.is_empty() {
                return Err(anyhow::Error::msg("empty result"));
            }
            anyhow::Ok(concat_batches(&batches[0].schema(), batches.iter())?)
        })?;

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
                    Array::Int32Array(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<Int64Array>() {
                    Array::Int64Array(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<Float32Array>() {
                    Array::Float32Array(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<Float64Array>() {
                    Array::Float64Array(array.values().to_vec())
                } else if let Some(array) = col.as_any().downcast_ref::<StringArray>() {
                    Array::TextArray((0..col.len()).map(|x| array.value(x).to_string()).collect())
                } else if let Some(array) = col.as_any().downcast_ref::<TimestampMicrosecondArray>()
                {
                    Array::Int64Array(array.values().to_vec())
                } else {
                    Array::Nil
                }
            })
            .collect::<Vec<_>>();
        Ok(probing_proto::prelude::DataFrame::new(names, columns))
    }

    pub fn execute<E: Into<String>>(self, query: &str, encoder: E) -> anyhow::Result<Vec<u8>> {
        futures::executor::block_on(async {
            let batches = self.sql(query).await?.collect().await?;
            let merged = concat_batches(&batches[0].schema(), batches.iter())?;
            chunked_encode(&merged, encoder)
        })
    }

    pub fn enable<S: AsRef<str>>(&self, domain: S, plugin: Arc<dyn Plugin>) -> Result<()> {
        let category = plugin.category();

        let catalog = if let Some(catalog) = self.context.catalog(domain.as_ref()) {
            catalog
        } else {
            self.context
                .register_catalog(domain.as_ref(), Arc::new(MemoryCatalogProvider::new()));
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
                catalog.schema(category.as_str())
            } else {
                let schema = MemorySchemaProvider::new();
                catalog.register_schema(category.as_str(), Arc::new(schema))?;
                catalog.schema(category.as_str())
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
    pub fn with_plugin<T: Into<String>>(mut self, namespace: T, plugin: Arc<dyn Plugin>) -> Self {
        let namespace = namespace.into();
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

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
