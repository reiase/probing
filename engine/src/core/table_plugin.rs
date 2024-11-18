use std::{any::Any, fmt::Debug, marker::PhantomData, sync::Arc};

use super::Plugin;
use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::catalog::{Session, TableProvider};
use datafusion::datasource::TableType;
use datafusion::error::Result;
use datafusion::physical_plan::{memory::MemoryExec, ExecutionPlan};
use datafusion::prelude::Expr;

pub trait CustomTable {
    fn name() -> &'static str;
    fn schema() -> SchemaRef;
    fn data() -> Vec<RecordBatch>;
}

pub struct TablePlugin<T: CustomTable> {
    name: String,
    category: String,

    data: PhantomData<T>,
}

impl<T: CustomTable> Default for TablePlugin<T> {
    fn default() -> Self {
        Self {
            name: T::name().to_string(),
            category: "probe".to_string(),
            data: Default::default(),
        }
    }
}

impl<T: CustomTable> TablePlugin<T> {
    pub fn new<S: Into<String>>(name: S, category: S) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            data: PhantomData::<T> {},
        }
    }
}

impl<T: CustomTable + Default + Debug + Send + Sync + 'static> Plugin for TablePlugin<T> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn kind(&self) -> super::PluginType {
        super::PluginType::TableProviderPlugin
    }

    fn category(&self) -> String {
        self.category.clone()
    }

    fn register_table(
        &self,
        schema: std::sync::Arc<dyn datafusion::catalog::SchemaProvider>,
        _state: &datafusion::execution::SessionState,
    ) -> datafusion::error::Result<()> {
        schema.register_table(self.name(), Arc::new(TableDataSource::<T>::default()))?;
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct TableDataSource<T: CustomTable> {
    data: PhantomData<T>,
}

#[async_trait]
impl<T: CustomTable + Default + Debug + Send + Sync + 'static> TableProvider
    for TableDataSource<T>
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        T::schema()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        // filters and limit can be used here to inject some push-down operations if needed
        _filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let data = T::data();
        let exec = MemoryExec::try_new(&[data], T::schema(), projection.cloned())?;
        Ok(Arc::new(exec))
    }
}
