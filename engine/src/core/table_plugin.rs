use std::{
    any::Any,
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    sync::Arc,
};

use super::Plugin;
use async_trait::async_trait;
use datafusion::{arrow::array::RecordBatch, error::Result, physical_plan::memory::MemoryStream};
use datafusion::{
    arrow::datatypes::{DataType, Field, Schema, SchemaRef},
    catalog::{Session, TableProvider},
    common::project_schema,
    datasource::TableType,
    physical_expr::EquivalenceProperties,
    physical_plan::{
        DisplayAs, DisplayFormatType, ExecutionMode, ExecutionPlan, Partitioning, PlanProperties,
    },
    prelude::Expr,
};

pub trait CustomTable {
    fn name() -> &'static str;
    fn schema() -> SchemaRef;
    fn data() -> Vec<RecordBatch>;
}

#[derive(Default)]
pub struct TablePlugin<T: CustomTable> {
    name: String,
    category: String,

    data: PhantomData<T>,
}

impl<T: CustomTable> TablePlugin<T> {
    pub fn new(name: String, category: String) -> Self {
        Self {
            name,
            category,
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
        state: &datafusion::execution::SessionState,
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
        Ok(Arc::new(TableDataSourceExec::<T>::new(
            projection,
            self.schema(),
        )))
    }
}

#[derive(Debug, Clone)]
struct TableDataSourceExec<T: Default> {
    projections: Option<Vec<usize>>,
    projected_schema: SchemaRef,
    cache: PlanProperties,
    data: PhantomData<T>,
}

impl<T: Default> TableDataSourceExec<T> {
    pub fn new(projections: Option<&Vec<usize>>, schema: SchemaRef) -> Self {
        let projected_schema = project_schema(&schema, projections).unwrap();
        let eq_properties = EquivalenceProperties::new(schema);
        let cache = PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(1),
            ExecutionMode::Bounded,
        );
        Self {
            projections: projections.cloned(),
            projected_schema,
            cache,
            data: PhantomData::<T> {},
        }
    }
}

impl<T: CustomTable + Default + 'static> DisplayAs for TableDataSourceExec<T> {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut Formatter) -> fmt::Result {
        write!(f, "Get Custom DataSource: {}", T::name())
    }
}

impl<T: CustomTable + Default + Debug + Send + Sync + 'static> ExecutionPlan
    for TableDataSourceExec<T>
{
    fn name(&self) -> &str {
        T::name()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.cache
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<datafusion::execution::TaskContext>,
    ) -> Result<datafusion::execution::SendableRecordBatchStream> {
        let rbs = T::data();
        let data = if let Some(projection) = self.projections.clone() {
            rbs.iter()
                .map(|rb| {
                    let cols = projection
                        .iter()
                        .map(|x| rb.column(*x).clone())
                        .collect::<Vec<_>>();
                    RecordBatch::try_new(self.projected_schema.clone(), cols).unwrap()
                })
                .collect()
        } else {
            rbs
        };
        Ok(Box::pin(MemoryStream::try_new(
            data,
            self.projected_schema.clone(),
            None,
        )?))
    }
}
