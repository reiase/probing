use std::path::Path;
use std::{any::Any, sync::Arc};

use async_trait::async_trait;
use datafusion::catalog::{CatalogProvider, SchemaProvider, TableProvider};
use datafusion::datasource::{
    file_format::csv::CsvFormat,
    listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl},
};
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::SessionState;
use datafusion::prelude::SessionContext;

use crate::core::{Plugin, PluginType};

#[derive(Default, Debug)]
pub struct FilesPlugin {}

impl Plugin for FilesPlugin {
    fn name(&self) -> String {
        "files".to_string()
    }

    fn kind(&self) -> PluginType {
        PluginType::SchemaProviderPlugin
    }

    fn category(&self) -> String {
        "files".to_string()
    }

    #[allow(unused)]
    fn register_schema(
        &self,
        catalog: Arc<dyn CatalogProvider>,
        state: &SessionState,
    ) -> Result<()> {
        catalog.register_schema("files", Arc::new(FileSystemSchema::default()));
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct FileSystemSchema {}

#[async_trait]
impl SchemaProvider for FileSystemSchema {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn table_names(&self) -> Vec<String> {
        let direntries = std::fs::read_dir(".").unwrap();
        direntries
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    let filename = entry.file_name().into_string().unwrap();
                    if filename.ends_with(".csv") {
                        Some(filename)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }
    async fn table(&self, name: &str) -> Result<Option<Arc<dyn TableProvider>>> {
        let ctx = SessionContext::new();
        let state = ctx.state();
        let table_path = ListingTableUrl::parse(name)?;
        let opts = ListingOptions::new(Arc::new(CsvFormat::default()));
        let conf = ListingTableConfig::new(table_path)
            .with_listing_options(opts)
            .infer_schema(&state)
            .await;
        let table = ListingTable::try_new(conf?)?;
        Ok(Some(Arc::new(table)))
    }

    #[allow(unused)]
    fn register_table(
        &self,
        name: String,
        table: Arc<dyn TableProvider>,
    ) -> Result<Option<Arc<dyn TableProvider>>> {
        Err(datafusion::error::DataFusionError::NotImplemented(
            "unable to create tables".to_string(),
        ))
    }
    #[allow(unused_variables)]
    fn deregister_table(&self, name: &str) -> Result<Option<Arc<dyn TableProvider>>> {
        Err(DataFusionError::NotImplemented(
            "unable to drop tables".to_string(),
        ))
    }

    fn table_exist(&self, name: &str) -> bool {
        let path = Path::new(name);
        path.exists()
    }
}
