# Probing 快速开始指南

Probing 是一个用于监控和跟踪 Python/PyTorch 程序的工具，可以帮助开发者更好地了解程序运行状态和性能表现。本指南将帮助你从零开始使用 Probing 工具。

## 1. 环境准备

在开始使用 Probing 之前，需要安装以下依赖：

```bash
# 安装 Rust 环境
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh 

# 安装 nightly 版本工具链
rustup toolchain install nightly 
rustup default nightly           

# 安装 WebAssembly 支持
rustup target add wasm32-unknown-unknown 

# 安装跨 glibc 版本构建支持
cargo install cargo-zigbuild 
pip install ziglang          # 简易安装方式，可能需要验证
```

## 2. 构建与安装

完成环境准备后，可以按照以下步骤构建和安装 Probing：

```bash
# 构建发布包
make ZIG=1

# 安装构建包（使用 force-reinstall 确保更新成功）
pip install dist/probing-0.2.0-py3-none-manylinux_2_12_x86_64.manylinux2010_x86_64.whl --force-reinstall 
```

## 3. 基本使用示例

安装完成后，可以使用以下命令测试 Probing 的功能：

```bash
# 简单测试 - 监控 ImageNet 训练过程
PROBE=1 python examples/test_imagenet.py -a resnet18 --dummy -b 1

# 高级测试 - 跟踪特定函数中的变量
PROBE_TORCH_EXPRS="loss@train,acc1@train" PROBE=1 python examples/test_imagenet.py -a resnet18 --dummy -b 1
```

## 4. 监控与数据查询

Probing 提供了一系列命令用于监控进程和查询收集的数据：

```bash
# 列出所有已被注入探针的进程
probing list 

# 查询特定进程中的数据表
probing <pid> query "show tables"

# 查询 PyTorch 模型追踪数据
probing <pid> query "select * from python.torch_trace"

# 查询被跟踪的变量
probing <pid> query "select * from python.variables"
```

## 5. 高级功能

除了基本的监控功能外，Probing 还支持：

- 变量追踪：通过设置环境变量 `PROBE_TORCH_EXPRS` 来指定要追踪的变量
- 实时监控：在程序运行过程中实时查看数据变化
- 自定义查询：使用类 SQL 语法进行灵活的数据查询

## 6. 故障排除

如果在使用过程中遇到问题：

- 确保所有依赖已正确安装
- 检查 Rust 工具链是否为 nightly 版本
- 使用 `--force-reinstall` 确保 Probing 被正确安装
