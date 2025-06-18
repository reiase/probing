use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::cluster_model::{NodeId, WorkerId};

/// 拓扑视图结构，包含版本和时效性信息
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TopologyView {
    /// 拓扑版本号，用于确保一致性
    pub version: u64,
    /// 拓扑更新时间戳
    pub timestamp: u64,
    /// 节点到worker的映射
    pub workers_per_node: HashMap<NodeId, Vec<WorkerId>>,
    /// 预估的总节点数（用于检测视图完整性）
    pub estimated_total_nodes: usize,
}

impl TopologyView {
    /// 创建新的拓扑视图
    pub fn new(workers_per_node: HashMap<NodeId, Vec<WorkerId>>, estimated_total: usize) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: now,
            timestamp: now,
            workers_per_node,
            estimated_total_nodes: estimated_total,
        }
    }

    /// 创建空的拓扑视图
    pub fn empty() -> Self {
        Self {
            version: 0,
            timestamp: 0,
            workers_per_node: HashMap::new(),
            estimated_total_nodes: 0,
        }
    }

    /// 使用指定版本创建拓扑视图
    pub fn with_version(
        workers_per_node: HashMap<NodeId, Vec<WorkerId>>,
        estimated_total: usize,
        version: u64,
    ) -> Self {
        Self {
            version,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            workers_per_node,
            estimated_total_nodes: estimated_total,
        }
    }

    /// 检查拓扑视图的完整性和时效性
    pub fn is_sufficient(&self, min_knowledge_ratio: f64, ttl_seconds: u64) -> bool {
        // 检查时效性
        if !self.is_fresh(ttl_seconds) {
            return false;
        }

        // 检查完整性
        self.is_complete(min_knowledge_ratio)
    }

    /// 检查拓扑视图是否新鲜（未过期）
    pub fn is_fresh(&self, ttl_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now - self.timestamp <= ttl_seconds
    }

    /// 检查拓扑视图是否完整
    pub fn is_complete(&self, min_knowledge_ratio: f64) -> bool {
        let known_nodes = self.workers_per_node.len();

        if self.estimated_total_nodes == 0 {
            return known_nodes > 0;
        }

        let ratio = known_nodes as f64 / self.estimated_total_nodes as f64;
        ratio >= min_knowledge_ratio
    }

    /// 获取所有节点ID
    pub fn get_node_ids(&self) -> Vec<NodeId> {
        self.workers_per_node.keys().cloned().collect()
    }

    /// 获取指定节点的worker列表
    pub fn get_workers(&self, node_id: &NodeId) -> Option<&Vec<WorkerId>> {
        self.workers_per_node.get(node_id)
    }

    /// 获取已知节点数量
    pub fn known_nodes_count(&self) -> usize {
        self.workers_per_node.len()
    }

    /// 获取总worker数量
    pub fn total_workers_count(&self) -> usize {
        self.workers_per_node
            .values()
            .map(|workers| workers.len())
            .sum()
    }

    /// 检查是否包含指定节点
    pub fn contains_node(&self, node_id: &NodeId) -> bool {
        self.workers_per_node.contains_key(node_id)
    }

    /// 更新拓扑视图的时间戳
    pub fn refresh_timestamp(&mut self) {
        self.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// 合并另一个拓扑视图
    pub fn merge(&mut self, other: &TopologyView) {
        // 使用更新的版本
        if other.version > self.version {
            self.version = other.version;
        }

        // 使用更新的时间戳
        if other.timestamp > self.timestamp {
            self.timestamp = other.timestamp;
        }

        // 合并节点信息
        for (node_id, workers) in &other.workers_per_node {
            self.workers_per_node
                .insert(node_id.clone(), workers.clone());
        }

        // 更新预估总节点数
        self.estimated_total_nodes = self.estimated_total_nodes.max(other.estimated_total_nodes);
    }

    /// 获取拓扑统计信息
    pub fn get_stats(&self) -> TopologyStats {
        TopologyStats {
            version: self.version,
            timestamp: self.timestamp,
            known_nodes: self.known_nodes_count(),
            total_workers: self.total_workers_count(),
            estimated_total_nodes: self.estimated_total_nodes,
            completeness_ratio: if self.estimated_total_nodes > 0 {
                self.known_nodes_count() as f64 / self.estimated_total_nodes as f64
            } else {
                1.0
            },
        }
    }
}

/// 拓扑统计信息
#[derive(Debug, Clone, PartialEq)]
pub struct TopologyStats {
    pub version: u64,
    pub timestamp: u64,
    pub known_nodes: usize,
    pub total_workers: usize,
    pub estimated_total_nodes: usize,
    pub completeness_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn create_test_workers_map() -> HashMap<NodeId, Vec<WorkerId>> {
        let mut workers_per_node = HashMap::new();
        workers_per_node.insert(
            "node1".to_string(),
            vec!["worker1".to_string(), "worker2".to_string()],
        );
        workers_per_node.insert("node2".to_string(), vec!["worker3".to_string()]);
        workers_per_node.insert(
            "node3".to_string(),
            vec![
                "worker4".to_string(),
                "worker5".to_string(),
                "worker6".to_string(),
            ],
        );
        workers_per_node
    }

    #[test]
    fn test_topology_view_creation() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node.clone(), 5);

        assert_eq!(topology.workers_per_node, workers_per_node);
        assert_eq!(topology.estimated_total_nodes, 5);
        assert!(topology.version > 0);
        assert!(topology.timestamp > 0);
    }

    #[test]
    fn test_empty_topology_view() {
        let topology = TopologyView::empty();

        assert_eq!(topology.version, 0);
        assert_eq!(topology.timestamp, 0);
        assert!(topology.workers_per_node.is_empty());
        assert_eq!(topology.estimated_total_nodes, 0);
    }

    #[test]
    fn test_topology_with_version() {
        let workers_per_node = create_test_workers_map();
        let version = 12345;
        let topology = TopologyView::with_version(workers_per_node.clone(), 3, version);

        assert_eq!(topology.version, version);
        assert_eq!(topology.workers_per_node, workers_per_node);
        assert_eq!(topology.estimated_total_nodes, 3);
    }

    #[test]
    fn test_is_fresh() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 3);

        // 应该是新鲜的
        assert!(topology.is_fresh(60)); // 60秒TTL
        assert!(topology.is_fresh(1)); // 1秒TTL

        // 创建一个过期的拓扑
        let mut old_topology = topology.clone();
        old_topology.timestamp = 0; // 设置为很早的时间
        assert!(!old_topology.is_fresh(1));
    }

    #[test]
    fn test_is_complete() {
        let workers_per_node = create_test_workers_map();

        // 测试完整性检查
        let topology = TopologyView::new(workers_per_node.clone(), 3); // 3个已知节点，预估3个总节点
        assert!(topology.is_complete(1.0)); // 100%完整性
        assert!(topology.is_complete(0.8)); // 80%完整性

        let topology2 = TopologyView::new(workers_per_node, 5); // 3个已知节点，预估5个总节点
        assert!(!topology2.is_complete(0.8)); // 60%完整性，不满足80%要求
        assert!(topology2.is_complete(0.5)); // 60%完整性，满足50%要求

        // 测试估计总数为0的情况
        let topology3 = TopologyView::new(HashMap::new(), 0);
        assert!(!topology3.is_complete(0.5)); // 没有已知节点

        let mut non_empty_map = HashMap::new();
        non_empty_map.insert("node1".to_string(), vec!["worker1".to_string()]);
        let topology4 = TopologyView::new(non_empty_map, 0);
        assert!(topology4.is_complete(0.5)); // 有已知节点，估计总数为0
    }

    #[test]
    fn test_is_sufficient() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 3);

        // 应该是足够的（新鲜且完整）
        assert!(topology.is_sufficient(1.0, 60));
        assert!(topology.is_sufficient(0.8, 60));

        // 创建一个过期的拓扑
        let mut old_topology = topology.clone();
        old_topology.timestamp = 0;
        assert!(!old_topology.is_sufficient(0.8, 1)); // 过期了

        // 创建一个不完整的拓扑
        let incomplete_topology = TopologyView::new(create_test_workers_map(), 10); // 3个已知，10个总计
        assert!(!incomplete_topology.is_sufficient(0.5, 60)); // 30%完整性，不满足50%要求
    }

    #[test]
    fn test_get_node_ids() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 3);

        let node_ids = topology.get_node_ids();
        assert_eq!(node_ids.len(), 3);
        assert!(node_ids.contains(&"node1".to_string()));
        assert!(node_ids.contains(&"node2".to_string()));
        assert!(node_ids.contains(&"node3".to_string()));
    }

    #[test]
    fn test_get_workers() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 3);

        let workers = topology.get_workers(&"node1".to_string());
        assert!(workers.is_some());
        assert_eq!(workers.unwrap().len(), 2);
        assert!(workers.unwrap().contains(&"worker1".to_string()));
        assert!(workers.unwrap().contains(&"worker2".to_string()));

        let no_workers = topology.get_workers(&"nonexistent".to_string());
        assert!(no_workers.is_none());
    }

    #[test]
    fn test_counts() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 5);

        assert_eq!(topology.known_nodes_count(), 3);
        assert_eq!(topology.total_workers_count(), 6); // 2 + 1 + 3
        assert_eq!(topology.estimated_total_nodes, 5);
    }

    #[test]
    fn test_contains_node() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 3);

        assert!(topology.contains_node(&"node1".to_string()));
        assert!(topology.contains_node(&"node2".to_string()));
        assert!(topology.contains_node(&"node3".to_string()));
        assert!(!topology.contains_node(&"node4".to_string()));
    }

    #[test]
    fn test_refresh_timestamp() {
        let workers_per_node = create_test_workers_map();
        let mut topology = TopologyView::new(workers_per_node, 3);

        let original_timestamp = topology.timestamp;

        // 等待一小段时间确保时间戳会改变
        thread::sleep(Duration::from_millis(10));

        topology.refresh_timestamp();
        assert!(topology.timestamp >= original_timestamp);
    }

    #[test]
    fn test_merge() {
        let mut workers1 = HashMap::new();
        workers1.insert("node1".to_string(), vec!["worker1".to_string()]);
        workers1.insert("node2".to_string(), vec!["worker2".to_string()]);

        let mut workers2 = HashMap::new();
        workers2.insert("node2".to_string(), vec!["worker2_updated".to_string()]);
        workers2.insert("node3".to_string(), vec!["worker3".to_string()]);

        let mut topology1 = TopologyView::with_version(workers1, 3, 100);
        let topology2 = TopologyView::with_version(workers2, 5, 200);

        topology1.merge(&topology2);

        // 检查版本更新
        assert_eq!(topology1.version, 200);

        // 检查节点合并
        assert_eq!(topology1.known_nodes_count(), 3);
        assert!(topology1.contains_node(&"node1".to_string()));
        assert!(topology1.contains_node(&"node2".to_string()));
        assert!(topology1.contains_node(&"node3".to_string()));

        // 检查预估总数更新
        assert_eq!(topology1.estimated_total_nodes, 5);

        // node2应该被更新
        assert_eq!(
            topology1.get_workers(&"node2".to_string()).unwrap(),
            &vec!["worker2_updated".to_string()]
        );
    }

    #[test]
    fn test_get_stats() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 5);

        let stats = topology.get_stats();

        assert_eq!(stats.known_nodes, 3);
        assert_eq!(stats.total_workers, 6);
        assert_eq!(stats.estimated_total_nodes, 5);
        assert!((stats.completeness_ratio - 0.6).abs() < f64::EPSILON); // 3/5 = 0.6
    }

    #[test]
    fn test_serde_serialization() {
        let workers_per_node = create_test_workers_map();
        let topology = TopologyView::new(workers_per_node, 3);

        // 测试序列化
        let serialized = serde_json::to_string(&topology).unwrap();
        assert!(!serialized.is_empty());

        // 测试反序列化
        let deserialized: TopologyView = serde_json::from_str(&serialized).unwrap();
        assert_eq!(topology.workers_per_node, deserialized.workers_per_node);
        assert_eq!(
            topology.estimated_total_nodes,
            deserialized.estimated_total_nodes
        );
        assert_eq!(topology.version, deserialized.version);
        assert_eq!(topology.timestamp, deserialized.timestamp);
    }
}
