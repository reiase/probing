use std::sync::Arc;

use datafusion::arrow::array::{GenericStringBuilder, RecordBatch};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};

use crate::core::{CustomTable, TablePlugin};

#[derive(Default, Debug)]
pub struct EnvTable {}

impl CustomTable for EnvTable {
    fn name() -> &'static str {
        "envs2"
    }

    fn schema() -> datafusion::arrow::datatypes::SchemaRef {
        SchemaRef::new(Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("value", DataType::Utf8, true),
        ]))
    }

    fn data() -> Vec<datafusion::arrow::array::RecordBatch> {
        let envs = std::env::vars();
        let mut names = GenericStringBuilder::<i32>::new();
        let mut values = GenericStringBuilder::<i32>::new();

        for env in envs {
            names.append_value(env.0.clone());
            values.append_value(env.1.clone());
        }

        let rbs = RecordBatch::try_new(
            Self::schema(),
            vec![Arc::new(names.finish()), Arc::new(values.finish())],
        );
        if let Ok(rbs) = rbs {
            vec![rbs]
        } else {
            Default::default()
        }
    }
}

pub type EnvPlugin = TablePlugin<EnvTable>;
