// probing/core/src/storage/entity.rs
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 通用的实体ID trait，所有ID类型都应该实现这个trait
pub trait EntityId:
    Clone
    + Send
    + Sync
    + std::fmt::Debug
    + std::fmt::Display
    + PartialEq
    + Eq
    + std::hash::Hash
    + 'static
{
    fn as_str(&self) -> &str;
    fn from_string(s: String) -> Self;
}

// 为我们的ID类型实现EntityId
impl EntityId for String {
    fn as_str(&self) -> &str {
        self // Corrected from self.as_str()
    }

    fn from_string(s: String) -> Self {
        s
    }
}

/// 可持久化实体的基础trait
/// 所有需要持久化的对象都应该实现这个trait
#[async_trait]
pub trait PersistentEntity:
    Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + 'static
{
    /// 实体的ID类型
    type Id: EntityId;

    /// 获取实体的唯一标识符
    fn id(&self) -> &Self::Id;

    /// 实体类型名称，用于存储路径/表名等
    fn entity_type() -> &'static str;

    /// 获取最后更新时间（如果有）
    fn last_updated(&self) -> Option<DateTime<Utc>> {
        None
    }

    /// 设置最后更新时间（如果支持）
    fn set_last_updated(&mut self, _timestamp: DateTime<Utc>) {
        // 默认空实现，实体可以选择性实现
    }

    /// 获取实体的版本（用于并发控制，可选）
    fn version(&self) -> Option<u64> {
        None
    }

    /// 获取相关的索引键，用于二级索引
    /// 返回格式: (index_name, index_key)
    fn index_keys(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

/// 通用的存储接口，支持任何实现了PersistentEntity的类型
#[async_trait]
pub trait EntityStore: Send + Sync + 'static {
    /// 保存实体
    async fn put<T: PersistentEntity>(&self, entity: &T) -> Result<()>;

    /// 根据ID获取实体
    async fn get<T: PersistentEntity>(&self, id: &T::Id) -> Result<Option<T>>;

    /// 删除实体
    async fn del<T: PersistentEntity>(&self, id: &T::Id) -> Result<()>;

    /// 列出某类型的所有实体（谨慎使用，可能返回大量数据）
    async fn list_all<T: PersistentEntity>(&self) -> Result<Vec<T>>;

    /// 分页列出实体
    async fn list_paginated<T: PersistentEntity>(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<T>, bool)>; // (entities, has_more)
}
