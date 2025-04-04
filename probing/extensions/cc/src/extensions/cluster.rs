use probing_core::core::cluster;
use probing_core::core::CustomTable;
use probing_core::core::TablePluginHelper;

use probing_core::core::ArrayRef;
use probing_core::core::DataType;
use probing_core::core::Field;
use probing_core::core::RecordBatch;
use probing_core::core::Schema;
use probing_core::core::SchemaRef;
use probing_core::core::TimeUnit;

#[derive(Default, Debug)]
pub struct ClusterTable {}

impl ClusterTable {}

impl CustomTable for ClusterTable {
    fn name() -> &'static str {
        "nodes"
    }

    fn schema() -> SchemaRef {
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
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Microsecond, None),
                false,
            ),
        ]))
    }

    fn data() -> Vec<RecordBatch> {
        let nodes = cluster::get_nodes();
        let mut fields: Vec<ArrayRef> = vec![];

        fields.push(cluster::extract_array(&nodes, |n| n.host.clone()));
        fields.push(cluster::extract_array(&nodes, |n| n.addr.clone()));
        fields.push(cluster::extract_array(&nodes, |n| n.local_rank));
        fields.push(cluster::extract_array(&nodes, |n| n.rank));
        fields.push(cluster::extract_array(&nodes, |n| n.world_size));
        fields.push(cluster::extract_array(&nodes, |n| n.group_rank));
        fields.push(cluster::extract_array(&nodes, |n| n.group_world_size));
        fields.push(cluster::extract_array(&nodes, |n| n.role_name.clone()));
        fields.push(cluster::extract_array(&nodes, |n| n.role_rank));
        fields.push(cluster::extract_array(&nodes, |n| n.role_world_size));
        fields.push(cluster::extract_array(&nodes, |n| n.status.clone()));
        fields.push(cluster::extract_array(&nodes, |n| {
            std::time::Duration::from_micros(n.timestamp)
        }));

        if let Ok(batches) = RecordBatch::try_new(Self::schema(), fields) {
            vec![batches]
        } else {
            Default::default()
        }
    }
}

pub type ClusterPlugin = TablePluginHelper<ClusterTable>;

use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;

#[derive(Debug, Default, EngineExtension)]
pub struct ClusterExtension {}

impl ClusterExtension {
    fn datasrc(
        &self,
        category: &str,
        name: Option<&str>,
    ) -> Option<std::sync::Arc<dyn probing_core::core::Plugin + Sync + Send>> {
        match name {
            Some(name) => Some(ClusterPlugin::create(category, name)),
            None => None,
        }
    }
}
