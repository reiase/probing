use std::{any::Any, fmt::Debug, marker::PhantomData, sync::Arc};

use super::Plugin;
use arrow::datatypes::{DataType, Field, Schema};
use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::catalog::{CatalogProvider, SchemaProvider, Session, TableProvider};
use datafusion::datasource::memory::DataSourceExec;
use datafusion::datasource::memory::MemorySourceConfig;
use datafusion::datasource::TableType;
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::SessionState;
use datafusion::physical_plan::ExecutionPlan;
use datafusion::prelude::Expr;

/// Trait defining a custom table with static/dynamic schema and data
///
/// Implement this to create tables that:
/// - Have a fixed name
/// - Use a predefined schema
pub trait CustomTable {
    /// Returns the table name (must be constant)
    fn name() -> &'static str;

    /// Returns the table schema
    fn schema() -> SchemaRef;

    /// Provides the data batches
    fn data() -> Vec<RecordBatch>;
}

/// Helper struct that bridges a CustomTable implementation with the Plugin system.
/// Handles registration and integration with DataFusion query engine.
pub struct TablePluginHelper<T: CustomTable> {
    /// Name of the table as it will be registered
    name: String,

    /// Namespace the table belongs to
    namespace: String,

    /// PhantomData to track the generic parameter T
    data: PhantomData<T>,
}

impl<T: CustomTable> Default for TablePluginHelper<T> {
    fn default() -> Self {
        Self {
            name: T::name().to_string(),
            namespace: "probe".to_string(),
            data: Default::default(),
        }
    }
}

/// Methods for constructing and working with TablePluginHelper instances
impl<T: CustomTable + std::default::Default + std::fmt::Debug + Send + Sync + 'static>
    TablePluginHelper<T>
{
    /// Creates a new TablePluginHelper with custom name and namespace
    pub fn new<S: Into<String>>(namespace: S, name: S) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
            data: PhantomData::<T> {},
        }
    }

    /// Factory method that creates a TablePluginHelper wrapped in an Arc
    /// Returns a trait object that can be used with the plugin system
    pub fn create<S: Into<String>>(namespace: S, name: S) -> Arc<dyn Plugin + Send + Sync> {
        Arc::new(Self::new(namespace, name))
    }
}

/// Implementation of the Plugin trait for TablePluginHelper
impl<T: CustomTable + Default + Debug + Send + Sync + 'static> Plugin for TablePluginHelper<T> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn kind(&self) -> super::PluginType {
        super::PluginType::Table
    }

    fn namespace(&self) -> String {
        self.namespace.clone()
    }

    /// Registers this table with the provided schema provider
    /// Links the CustomTable implementation with DataFusion's query engine
    fn register_table(
        &self,
        schema: std::sync::Arc<dyn datafusion::catalog::SchemaProvider>,
        _state: &datafusion::execution::SessionState,
    ) -> datafusion::error::Result<()> {
        schema.register_table(self.name(), Arc::new(TableDataSource::<T>::default()))?;
        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
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
        let srccfg = MemorySourceConfig::try_new(&[data], T::schema(), projection.cloned())?;
        let exec = DataSourceExec::new(Arc::new(srccfg));
        Ok(Arc::new(exec))
    }
}

#[derive(Default, Debug)]
pub struct LazyTableSource {
    pub name: String,
    pub schema: Option<SchemaRef>,
    pub data: Vec<RecordBatch>,
}

#[async_trait]
impl TableProvider for LazyTableSource {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        if let Some(schema) = &self.schema {
            return schema.clone();
        }
        SchemaRef::new(Schema::new(vec![Field::new(
            "unknown_fields",
            DataType::Int64,
            false,
        )]))
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
        let data = &self.data;
        if data.is_empty() {
            return Err(DataFusionError::Execution(
                "no data found for lazy table".to_string(),
            ));
        }
        let schema = data[0].schema();
        let srccfg =
            MemorySourceConfig::try_new(std::slice::from_ref(data), schema, projection.cloned())?;
        let exec = DataSourceExec::new(Arc::new(srccfg));
        Ok(Arc::new(exec))
    }
}

/// Trait for implementing a custom namespace that can dynamically generate tables
/// Provides a mechanism for on-demand table creation based on name/expression
#[allow(unused)]
#[async_trait]
pub trait CustomNamespace: Sync + Send {
    /// Returns the name of the namespace
    fn name() -> &'static str;

    /// Returns a list of available table names in this namespace
    fn list() -> Vec<String>;

    /// Generates data for a specific table expression
    /// Default implementation returns empty data
    fn data(expr: &str) -> Vec<RecordBatch> {
        vec![]
    }

    /// Creates a LazyTableSource for this namespace with the given expression
    fn make_lazy(expr: &str) -> Arc<LazyTableSource>
    where
        Self: Sized,
    {
        Arc::new(LazyTableSource {
            name: expr.to_string(),
            schema: None,
            data: Default::default(),
        })
    }

    /// Factory method to create a TableProvider for a specific table expression
    /// Used by the namespace provider to generate tables on demand
    async fn table(expr: String) -> Result<Option<Arc<dyn TableProvider>>>
    where
        Self: Default + Debug + Send + Sync + Sized + 'static,
    {
        // let lazy = Arc::new(LazyTableSource::<Self> {
        //     name: expr.clone(),
        //     schema: None,
        //     data: Default::default(),
        // });
        let lazy = Self::make_lazy(expr.as_str());
        Ok(Some(lazy))
    }
}

/// Helper struct that bridges a CustomNamespace implementation with the Plugin system
/// Manages registration and integration with DataFusion query engine
pub struct NamespacePluginHelper<T: CustomNamespace> {
    /// Namespace the schema belongs to
    namespace: String,

    /// PhantomData to track the generic parameter T
    data: PhantomData<T>,
}

impl<T: CustomNamespace> Default for NamespacePluginHelper<T> {
    fn default() -> Self {
        Self {
            namespace: "probe".to_string(),
            data: Default::default(),
        }
    }
}

impl<T: CustomNamespace + std::default::Default + std::fmt::Debug + Send + Sync + 'static>
    NamespacePluginHelper<T>
{
    pub fn new<S: Into<String>>(namespace: S) -> Self {
        Self {
            namespace: namespace.into(),
            data: PhantomData::<T> {},
        }
    }

    pub fn create<S: Into<String>>(namespace: S) -> Arc<dyn Plugin + Send + Sync> {
        Arc::new(Self::new(namespace))
    }
}

impl<T: CustomNamespace + Default + Debug + Send + Sync + 'static> Plugin
    for NamespacePluginHelper<T>
{
    fn name(&self) -> String {
        self.namespace.clone()
    }

    fn kind(&self) -> super::PluginType {
        super::PluginType::Namespace
    }

    fn namespace(&self) -> String {
        self.namespace.clone()
    }

    #[allow(unused)]
    fn register_namespace(
        &self,
        catalog: Arc<dyn CatalogProvider>,
        state: &SessionState,
    ) -> Result<()> {
        catalog.register_schema(
            self.namespace().as_str(),
            Arc::new(CustomNamespaceDataSource::<T>::default()),
        );
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct CustomNamespaceDataSource<T: CustomNamespace> {
    data: PhantomData<T>,
}

#[async_trait]
impl<T: CustomNamespace + Default + Debug + Send + Sync + 'static> SchemaProvider
    for CustomNamespaceDataSource<T>
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
