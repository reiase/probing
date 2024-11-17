use std::sync::Arc;

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

#[allow(unused)]
pub struct Engine {
    context: SessionContext,
    // plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl Engine {
    pub fn new() -> Self {
        let config = SessionConfig::default()
            .with_information_schema(true)
            .with_default_catalog_and_schema("probe", "probe");
        let engine = Engine {
            context: SessionContext::new_with_config(config),
            // plugins: Default::default(),
        };

        engine
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
            plugin.register_schema(catalog, &state)?;
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
        }
        Ok(())
    }
}
