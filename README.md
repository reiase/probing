# probing - 分布式AI应用性能探针

![probing demo](docs/images/demo.gif)

> Uncover the Hidden Truth of AI Performance

probing让你轻松洞察分布式AI应用的性能瓶颈。它简单、非侵入，专注于解决AI开发中最核心的性能分析难题。

### What probing does...

- **让你看见AI应用的真实运行状态** - 代码卡在哪里，一目了然
- **让复杂的分布式应用变得透明** - 跨节点对比堆栈、性能，找出瓶颈  
- **让性能分析变得常态化** - 全程性能监控，而非事后补救

### In contrast with traditional profilers, probing does not...

- **不依赖代码修改** - 无需在训练脚本中添加性能埋点
- **不影响训练效率** - 监控开销可忽略不计，训练该多快还多快
- **不仅限于事后分析** - 提供实时洞察，而非静态报告

[![PyPI version](https://badge.fury.io/py/probing.svg)](https://badge.fury.io/py/probing)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Downloads](https://pepy.tech/badge/probing)](https://pepy.tech/project/probing)

## 快速开始

### 安装

```bash
pip install probing
```

### 30秒上手

**事先启动（推荐）：**
```bash
# 在启动训练时开启监控
PROBING=1 python examples/imagenet.py -a resnet18 --dummy -b 1
```

**事后注入：**
```bash
# 1. 启动训练脚本
python examples/imagenet.py -a resnet18 --dummy -b 1 &
TRAIN_PID=$!

# 2. 注入性能监控
probing $TRAIN_PID inject
```

**堆栈分析：**

```bash
# 单机推荐
probing $TRAIN_PID backtrace

# 多机分布式，使用SQL处理
probing $TRAIN_PID "SELECT * FROM python.backtrace"
```

## 核心特性

- **动态探针注入** - 支持运行时探针注入，无需预先修改目标应用代码
- **分布式性能聚合** - 提供跨节点性能数据统一收集与关联分析能力
- **标准SQL查询接口** - 基于Apache DataFusion引擎，支持标准SQL语法进行性能数据查询
- **低开销监控** - 采用高效采样策略，监控开销控制在1%以内

## 基本用法

```bash
# 注入性能监控
probing <pid> inject

# 查看堆栈信息
probing <pid> backtrace

# 分析内存使用
probing <pid> memory

# 生成火焰图
probing <pid> flamegraph

# 复杂查询使用SQL
probing <pid> query "SELECT * FROM memory_usage WHERE timestamp > now() - interval '5 min'"
```

## 使用场景

### 训练性能分析
```bash
# 启动训练并同时监控
probing run python train.py

# 分析GPU利用率
probing query "SELECT avg(gpu_util) FROM gpu_metrics WHERE timestamp > now() - 60"
```

### 分布式训练诊断
```bash
# 监控所有节点
probing cluster attach

# 查看节点间通信
probing query "SELECT src_rank, dst_rank, avg(latency_ms) FROM comm_metrics"
```

### 内存分析
```bash
# 快速查看内存使用
probing <pid> memory

# 详细分析内存趋势（使用SQL）
probing <pid> query "SELECT hour(timestamp), avg(memory_mb) FROM memory_usage GROUP BY hour(timestamp)"
```

## 高级功能

### SQL查询接口
```bash
# 复杂性能分析
probing query "
  SELECT operation_name, avg(duration_ms), count(*)
  FROM profiling_data 
  WHERE timestamp > now() - interval '5 minutes'
  GROUP BY operation_name
  ORDER BY avg(duration_ms) DESC
"

# 训练进度跟踪
probing query "
  SELECT epoch, avg(loss), min(loss), count(*) as steps
  FROM training_logs 
  GROUP BY epoch 
  ORDER BY epoch
"
```

### 配置选项
```bash
# 设置采样率
export PROBING_SAMPLE_RATE=0.1

# 配置数据保留时间
export PROBING_RETENTION_DAYS=7

# 自定义探针配置
probing attach <pid> --config custom_probes.yaml
```

## 技术架构

Probing使用动态探针注入技术，在运行时监控Python进程：

```
训练进程 → 探针注入 → 数据收集 → SQL查询
python    probing     采集器      分析引擎
```

核心组件：
- **探针引擎** - 基于Rust的高性能数据采集
- **查询引擎** - 基于Apache DataFusion的SQL分析
- **存储层** - 列式存储，支持时序数据压缩
- **分布式协调** - 自动发现和聚合多节点数据

## 文档

- [用户指南](docs/user-guide.md) - 完整使用教程
- [API文档](docs/api-reference.md) - 详细接口说明  
- [示例代码](examples/) - 各种使用场景示例
- [GitHub Issues](https://github.com/reiase/probing/issues) - 问题反馈

## 开发构建

```bash
# 克隆代码
git clone https://github.com/reiase/probing.git
cd probing

# 构建项目 (需要Rust 1.70+)
make build

# 运行测试
make test

# 贡献代码
# 请先阅读 CONTRIBUTING.md
```

## 许可证

[Apache License 2.0](LICENSE)
