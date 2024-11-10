use std::any::Any;
use std::fmt::{self, Formatter};
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::array::{GenericStringBuilder, RecordBatch};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::catalog::{SchemaProvider, Session, TableProvider};
use datafusion::common::project_schema;
use datafusion::datasource::TableType;
use datafusion::error::Result;
use datafusion::execution::SessionState;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::memory::MemoryStream;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionMode, ExecutionPlan, Partitioning, PlanProperties,
};
use datafusion::prelude::Expr;

use crate::core::Plugin;
use crate::core::PluginType;

#[derive(Default)]
pub struct EnvPlugin {}

impl Plugin for EnvPlugin {
    fn name(&self) -> String {
        "envs".to_string()
    }

    fn kind(&self) -> PluginType {
        PluginType::TableProviderPlugin
    }

    fn category(&self) -> String {
        "process".to_string()
    }

    fn register_table(&self, schema: Arc<dyn SchemaProvider>, state: &SessionState) -> Result<()> {
        schema.register_table(self.name(), Arc::new(EnvDataSource::default()))?;
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct EnvDataSource {}

#[async_trait]
impl TableProvider for EnvDataSource {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        SchemaRef::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("value", DataType::Utf8, true),
        ]))
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
        Ok(Arc::new(EnvDataSourceExec::new(projection, self.schema())))
    }
}

#[derive(Debug, Clone)]
struct EnvDataSourceExec {
    projected_schema: SchemaRef,
    cache: PlanProperties,
}

impl EnvDataSourceExec {
    pub fn new(projections: Option<&Vec<usize>>, schema: SchemaRef) -> Self {
        let projected_schema = project_schema(&schema, projections).unwrap();
        let eq_properties = EquivalenceProperties::new(schema);
        let cache = PlanProperties::new(
            eq_properties,
            Partitioning::UnknownPartitioning(1),
            ExecutionMode::Bounded,
        );
        Self {
            projected_schema,
            cache,
        }
    }
}

impl DisplayAs for EnvDataSourceExec {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut Formatter) -> fmt::Result {
        write!(f, "Get Envirenment Variables")
    }
}

impl ExecutionPlan for EnvDataSourceExec {
    fn name(&self) -> &str {
        "Get Envirenment Variables"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &datafusion::physical_plan::PlanProperties {
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
        let envs = std::env::vars().collect::<Vec<_>>();

        let mut names = GenericStringBuilder::<i32>::new();
        let mut values = GenericStringBuilder::<i32>::new();

        for env in envs {
            names.append_value(env.0);
            values.append_value(env.1);
        }

        Ok(Box::pin(MemoryStream::try_new(
            vec![RecordBatch::try_new(
                self.projected_schema.clone(),
                vec![Arc::new(names.finish()), Arc::new(values.finish())],
            )?],
            self.schema(),
            None,
        )?))
    }
}
