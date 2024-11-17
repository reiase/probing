use std::sync::Arc;

use datafusion::arrow::{
    array::{GenericStringBuilder, RecordBatch, TimestampMicrosecondBuilder},
    datatypes::{DataType, Field, Schema, SchemaRef, TimeUnit},
};
use rmesg;

use crate::core::{CustomTable, TablePlugin};

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
        let entries = rmesg::log_entries(rmesg::Backend::KLogCtl, false).unwrap();
        let mut timestamp = TimestampMicrosecondBuilder::new();
        let mut facility = GenericStringBuilder::<i32>::new();
        let mut level = GenericStringBuilder::<i32>::new();
        let mut message = GenericStringBuilder::<i32>::new();

        for entry in entries {
            let ts = entry.timestamp_from_system_start;
            timestamp.append_value(ts.unwrap_or_default().as_micros() as i64);
            facility.append_value(
                entry
                    .facility
                    .unwrap_or(rmesg::entry::LogFacility::User)
                    .to_string(),
            );
            level.append_value(
                entry
                    .level
                    .unwrap_or(rmesg::entry::LogLevel::Info)
                    .to_string(),
            );
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

pub type KMsgPlugin = TablePlugin<KMsgTable>;