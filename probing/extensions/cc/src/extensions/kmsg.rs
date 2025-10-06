use std::sync::Arc;

use datafusion::arrow::array::{GenericStringBuilder, RecordBatch, TimestampMicrosecondBuilder};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef, TimeUnit};
use rmesg::entry::{LogFacility, LogLevel};
use rmesg::log_entries;
use rmesg::Backend;

use probing_core::core::{CustomTable, EngineCall, EngineDatasource, TablePluginHelper};

#[derive(Default, Debug)]
pub struct KMsgTable {}

impl CustomTable for KMsgTable {
    fn name() -> &'static str {
        "kmsg"
    }

    fn schema() -> datafusion::arrow::datatypes::SchemaRef {
        SchemaRef::new(Schema::new(vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Microsecond, None),
                false,
            ),
            Field::new("facility", DataType::Utf8, false),
            Field::new("level", DataType::Utf8, true),
            Field::new("message", DataType::Utf8, true),
        ]))
    }

    fn data() -> Vec<datafusion::arrow::array::RecordBatch> {
        let entries = log_entries(Backend::KLogCtl, false).unwrap();
        let mut timestamp = TimestampMicrosecondBuilder::new();
        let mut facility = GenericStringBuilder::<i32>::new();
        let mut level = GenericStringBuilder::<i32>::new();
        let mut message = GenericStringBuilder::<i32>::new();

        // let boot_time = match procfs::boot_time() {
        //     Ok(time) => time,
        //     Err(_) => return vec![],
        // };
        // let boot_time_micro = boot_time.timestamp_micros();

        for entry in entries {
            let ts = entry.timestamp_from_system_start;
            timestamp.append_value(
                ts.unwrap_or_default().as_micros() as i64, /* + boot_time_micro*/
            );
            facility.append_value(entry.facility.unwrap_or(LogFacility::User).to_string());
            level.append_value(entry.level.unwrap_or(LogLevel::Info).to_string());
            message.append_value(entry.message);
        }

        let rbs = RecordBatch::try_new(
            Self::schema(),
            vec![
                Arc::new(timestamp.finish()),
                Arc::new(facility.finish()),
                Arc::new(level.finish()),
                Arc::new(message.finish()),
            ],
        );
        if let Ok(rbs) = rbs {
            vec![rbs]
        } else {
            vec![]
        }
    }
}

pub type KMsgPlugin = TablePluginHelper<KMsgTable>;

use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;

#[derive(Debug, Default, EngineExtension)]
pub struct KMsgExtension {}

impl EngineCall for KMsgExtension {}

impl EngineDatasource for KMsgExtension {
    fn datasrc(
        &self,
        namespace: &str,
        name: Option<&str>,
    ) -> Option<std::sync::Arc<dyn probing_core::core::Plugin + Sync + Send>> {
        match name {
            Some(name) => Some(KMsgPlugin::create(namespace, name)),
            None => None,
        }
    }
}
