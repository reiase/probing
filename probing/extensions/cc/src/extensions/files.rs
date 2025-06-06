use std::sync::Arc;

use async_trait::async_trait;
use datafusion::catalog::TableProvider;
use datafusion::datasource::{
    file_format::csv::CsvFormat,
    listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl},
};
use datafusion::error::Result;
use datafusion::prelude::SessionContext;

use probing_core::core::{CustomNamespace, EngineCall, EngineDatasource, NamespacePluginHelper};

#[derive(Default, Debug)]
pub struct FileList {}

#[async_trait]
impl CustomNamespace for FileList {
    fn name() -> &'static str {
        "file"
    }

    fn list() -> Vec<String> {
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

    async fn table(expr: String) -> Result<Option<Arc<dyn TableProvider>>> {
        let ctx = SessionContext::new();
        let state = ctx.state();
        let table_path = ListingTableUrl::parse(expr)?;
        let opts = ListingOptions::new(Arc::new(CsvFormat::default()));
        let conf = ListingTableConfig::new(table_path)
            .with_listing_options(opts)
            .infer_schema(&state)
            .await;
        let table = ListingTable::try_new(conf?)?;
        Ok(Some(Arc::new(table)))
    }
}

pub type FilesPlugin = NamespacePluginHelper<FileList>;

use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;

#[derive(Debug, Default, EngineExtension)]
pub struct FilesExtension {}

impl EngineCall for FilesExtension {}

#[allow(unused)]
impl EngineDatasource for FilesExtension {
    fn datasrc(
        &self,
        namespace: &str,
        name: Option<&str>,
    ) -> Option<std::sync::Arc<dyn probing_core::core::Plugin + Sync + Send>> {
        match name {
            Some(name) => Some(FilesPlugin::create(namespace)),
            None => None,
        }
    }
}
