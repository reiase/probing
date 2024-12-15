use std::{any::Any, fmt::Debug, marker::PhantomData, sync::Arc};

use super::Plugin;
use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::catalog::{CatalogProvider, SchemaProvider, Session, TableProvider};
use datafusion::datasource::{MemTable, TableType};
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::SessionState;
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

#[allow(unused)]
#[async_trait]
pub trait CustomSchema: Sync + Send {
    fn name() -> &'static str;
    fn list() -> Vec<String>;
    fn data(expr: &str) -> Vec<RecordBatch> {
        vec![]
    }
    async fn table(expr: String) -> Result<Option<Arc<dyn TableProvider>>> {
        let data = Self::data(expr.as_str());
        if !data.is_empty() {
            let data = MemTable::try_new(data[0].schema(), vec![data]).ok();
            let data = data.map(|x| Arc::new(x) as Arc<dyn TableProvider>);
            Ok(data)
        } else {
            Ok(None)
        }
    }
}

pub struct SchemaPlugin<T: CustomSchema> {
    category: String,

    data: PhantomData<T>,
}

impl<T: CustomSchema> Default for SchemaPlugin<T> {
    fn default() -> Self {
        Self {
            category: "probe".to_string(),
            data: Default::default(),
        }
    }
}

impl<T: CustomSchema> SchemaPlugin<T> {
    pub fn new<S: Into<String>>(category: S) -> Self {
        Self {
            category: category.into(),
            data: PhantomData::<T> {},
        }
    }
}

impl<T: CustomSchema + Default + Debug + Send + Sync + 'static> Plugin for SchemaPlugin<T> {
    fn name(&self) -> String {
        self.category.clone()
    }

    fn kind(&self) -> super::PluginType {
        super::PluginType::SchemaProviderPlugin
    }

    fn category(&self) -> String {
        self.category.clone()
    }

    #[allow(unused)]
    fn register_schema(
        &self,
        catalog: Arc<dyn CatalogProvider>,
        state: &SessionState,
    ) -> Result<()> {
        catalog.register_schema(
            self.category().as_str(),
            Arc::new(CustomSchemaDataSource::<T>::default()),
        );
        Ok(())
    }
}

#[derive(Default)]
pub struct CustomSchemaDataSource<T: CustomSchema> {
    data: PhantomData<T>,
}

#[async_trait]
impl<T: CustomSchema + Default + Debug + Send + Sync + 'static> SchemaProvider
    for CustomSchemaDataSource<T>
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn table_names(&self) -> Vec<String> {
        T::list()
    }

    async fn table(&self, name: &str) -> Result<Option<Arc<dyn TableProvider>>> {
        T::table(name.to_string()).await
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
    fn table_exist(&self, _name: &str) -> bool {
        true
    }
}
