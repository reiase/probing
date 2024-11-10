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

    fn register_table(&self, schema: Arc<dyn SchemaProvider>, _: &SessionState) -> Result<()> {
        schema.register_table(self.name(), Arc::new(EnvDataSource::default()))?;
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct EnvDataSource {}

impl EnvDataSource {
    // pub fn is_supported(&self, filter: &Expr) -> bool {
    //     match filter {
    //         Expr::BinaryExpr(BinaryExpr {
    //             left: col,
    //             op: Operator::Eq,
    //             right: _,
    //         }) => match *col.clone() {
    //             Expr::Column(Column { relation: _, name }) => name.eq("name"),
    //             _ => false,
    //         },
    //         _ => false,
    //     }
    // }
}

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

    // fn supports_filters_pushdown(
    //     &self,
    //     filters: &[&Expr],
    // ) -> Result<Vec<TableProviderFilterPushDown>> {
    //     Ok(filters
    //         .iter()
    //         .map(|f| {
    //             if self.is_supported(f) {
    //                 TableProviderFilterPushDown::Inexact
    //             } else {
    //                 TableProviderFilterPushDown::Unsupported
    //             }
    //         })
    //         .collect())
    // }

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
        Ok(Arc::new(
            EnvDataSourceExec::new(projection, self.schema())
                .filters(&_filters)
                .limit(_limit),
        ))
    }
}

#[derive(Debug, Clone)]
struct EnvDataSourceExec {
    projected_schema: SchemaRef,
    filters: Vec<Expr>,
    limit: Option<usize>,
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
            filters: Default::default(),
            limit: Default::default(),
        }
    }
    pub fn filters(mut self, exprs: &[Expr]) -> Self {
        self.filters = exprs.iter().map(|x| x.clone()).collect::<Vec<_>>();
        self
    }
    pub fn limit(mut self, value: Option<usize>) -> Self {
        self.limit = value;
        self
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
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<datafusion::execution::TaskContext>,
    ) -> Result<datafusion::execution::SendableRecordBatchStream> {
        let envs = std::env::vars();
        // let envs = envs.iter();
        // for filter in self.filters.iter() {
        //     println!("filters: {}", filter);
        // }

        let envs = if let Some(limit) = self.limit {
            envs.take(limit).collect::<Vec<_>>()
        } else {
            envs.collect::<Vec<_>>()
        };

        let mut names = GenericStringBuilder::<i32>::new();
        let mut values = GenericStringBuilder::<i32>::new();

        for env in envs {
            names.append_value(env.0.clone());
            values.append_value(env.1.clone());
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
