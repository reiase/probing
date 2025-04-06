use std::sync::Arc;

use datafusion::arrow::array::{GenericStringBuilder, RecordBatch};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};

use probing_core::core::{CustomTable, TablePluginHelper};

#[derive(Default, Debug)]
pub struct EnvTable {}

impl CustomTable for EnvTable {
    fn name() -> &'static str {
        "envs"
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

pub type EnvPlugin = TablePluginHelper<EnvTable>;

use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;

#[derive(Debug, Default, EngineExtension)]
pub struct EnvExtension {}

impl EnvExtension {
    fn plugin(
        &self,
        namespace: &str,
        name: Option<&str>,
    ) -> Option<std::sync::Arc<dyn probing_core::core::Plugin + Sync + Send>> {
        match name {
            Some(name) => Some(EnvPlugin::create(namespace, name)),
            None => None,
        }
    }
}
