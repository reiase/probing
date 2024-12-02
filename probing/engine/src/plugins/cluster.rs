pub mod service;

use arrow::datatypes::TimeUnit;
use service::extract_array;
use service::get_nodes;

use arrow::array::{ArrayRef, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

use crate::core::CustomTable;
use crate::core::TablePlugin;

#[derive(Default, Debug)]
pub struct ClusterTable {}

impl ClusterTable {}

impl CustomTable for ClusterTable {
    fn name() -> &'static str {
        "nodes"
    }

    fn schema() -> arrow::datatypes::SchemaRef {
        SchemaRef::new(Schema::new(vec![
            Field::new("host", DataType::Utf8, false),
            Field::new("addr", DataType::Utf8, false),
            Field::new("local_rank", DataType::Int32, true),
            Field::new("rank", DataType::Int32, true),
            Field::new("world_size", DataType::Int32, true),
            Field::new("group_rank", DataType::Int32, true),
            Field::new("group_world_size", DataType::Int32, true),
            Field::new("role_name", DataType::Utf8, true),
            Field::new("role_rank", DataType::Int32, true),
            Field::new("role_world_size", DataType::Int32, true),
            Field::new("status", DataType::Utf8, true),
            Field::new("timestamp", DataType::Timestamp(TimeUnit::Microsecond, None), false),
        ]))
    }

    fn data() -> Vec<arrow::array::RecordBatch> {
        let nodes = get_nodes();
        let mut fields: Vec<ArrayRef> = vec![];

        fields.push(extract_array(&nodes, |n| n.host.clone()));
        fields.push(extract_array(&nodes, |n| n.addr.clone()));
        fields.push(extract_array(&nodes, |n| n.local_rank));
        fields.push(extract_array(&nodes, |n| n.rank));
        fields.push(extract_array(&nodes, |n| n.world_size));
        fields.push(extract_array(&nodes, |n| n.group_rank));
        fields.push(extract_array(&nodes, |n| n.group_world_size));
        fields.push(extract_array(&nodes, |n| n.role_name.clone()));
        fields.push(extract_array(&nodes, |n| n.role_rank));
        fields.push(extract_array(&nodes, |n| n.role_world_size));
        fields.push(extract_array(&nodes, |n| n.status.clone()));
        fields.push(extract_array(&nodes, |n| std::time::Duration::from_micros(n.timestamp as u64)));

        if let Ok(batches) = RecordBatch::try_new(Self::schema(), fields) {
            vec![batches]
        } else {
            Default::default()
        }
    }
}

pub type ClusterPlugin = TablePlugin<ClusterTable>;
