use std::sync::Arc;

use async_trait::async_trait;
use datafusion::catalog::TableProvider;
use datafusion::datasource::{
    file_format::csv::CsvFormat,
    listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl},
};
use datafusion::error::Result;
use datafusion::prelude::SessionContext;

use probing_core::core::{CustomSchema, SchemaPluginHelper};

#[derive(Default, Debug)]
pub struct FileList {}

#[async_trait]
impl CustomSchema for FileList {
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

pub type FilesPlugin = SchemaPluginHelper<FileList>;
