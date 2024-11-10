use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use datafusion::catalog::{CatalogProvider, SchemaProvider};
use datafusion::catalog_common::{MemoryCatalogProvider, MemorySchemaProvider};
use datafusion::error::Result;
use datafusion::execution::SessionState;
use datafusion::prelude::{DataFrame, SessionConfig, SessionContext};

#[derive(PartialEq, Eq)]
pub enum PluginType {
    TableProviderPlugin,
    SchemaProviderPlugin,
}
pub trait Plugin {
    fn name(&self) -> String;
    fn kind(&self) -> PluginType;
    fn category(&self) -> String;
    fn register_table(&self, schema: Arc<dyn SchemaProvider>, state: &SessionState) -> Result<()> {
        Ok(())
    }
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
            .with_default_catalog_and_schema("probe", "prove");
        let engine = Engine {
            context: SessionContext::new_with_config(config),
            plugins: Default::default(),
        };

        engine
    }

    pub async fn sql(self, query: &str) -> Result<DataFrame> {
        self.context.sql(query).await
    }

    pub fn enable(&self, catalog_name: String, plugin: Arc<dyn Plugin>) -> Result<()> {
        let category = plugin.category();

        let catalog = if self.context.catalog_names().contains(&catalog_name) {
            self.context.catalog(catalog_name.as_str())
        } else {
            let catalog = MemoryCatalogProvider::new();
            self.context
                .register_catalog(category.clone(), Arc::new(catalog));
            self.context.catalog(catalog_name.as_str())
        }
        .unwrap();
        if plugin.kind() == PluginType::SchemaProviderPlugin {
            let state: SessionState = self.context.state();
            plugin.register_schema(catalog, &state);
        } else if plugin.kind() == PluginType::TableProviderPlugin {
            let schema = if catalog.schema_names().contains(&category) {
                catalog.schema(&category.as_str())
            } else {
                let schema = MemorySchemaProvider::new();
                catalog.register_schema(category.as_str(), Arc::new(schema))?;
                catalog.schema(&category.as_str())
            };
            let state: SessionState = self.context.state();
            plugin.register_table(schema.unwrap(), &state);
        }
        Ok(())
    }
}
