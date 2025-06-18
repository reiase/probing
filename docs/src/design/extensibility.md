# Extensibility Framework

Probing的核心能力在于，它既能深入目标进程获取关键的性能与故障数据，也能灵活地向目标进程植入诊断逻辑与定制代码。为了满足多样化的监控与分析需求，用户可以通过其强大的扩展系统来定制和增强Probing的功能，从而更有效地解决特定场景下的问题。

Probing提供了两种主要的扩展路径：基于Python的轻量级扩展与基于Rust的高性能扩展。Python扩展以其无需重新构建Probing的便捷性，在实际生产环境中展现出色的灵活性；而Rust扩展则为开发者提供了更接近底层的控制能力和更高的执行效率，适用于对性能有极致要求或需要深度集成系统的场景。

## 基于Python扩展Probing

### SQL直接调用python

Probing支持在SQL查询中直接调用Python函数或者变量，作为一个快速的外部表：

```bash
# 调用Python标准库函数
probing $ENDPOINT query "SELECT * from python.`time.time()`"


# 调用自定义包函数
probing $ENDPOINT query "SELECT * FROM python.`pkg.some_func()`"
```

在`python`这个namespace下，通过反引号"`"来引用一段python代码，Probing会将该代码执行，并将结果作为表返回给接下来的SQL来执行。

### Python写入Probing

Probing支持使用`dataclass`自定义数据表：

```python
from dataclasses import dataclass
from probing.core import table

@table
@dataclass
class MetricData:
    timestamp: int
    cpu_usage: float
    memory_mb: int
    process_name: str

# 写入数据
MetricData.append(MetricData(1234567890, 85.2, 1024, "python"))

# 或者更加直接
MetricData(1234567890, 85.2, 1024, "python").save()
```
写入的数据可以通过SQL来查询
```bash
probing $ENDPOINT query "SELECT * FROM python.metric_data"
```

## Rust扩展系统

使用Rust语言，开发者可以更直接、更深入地扩展Probing的数据查询与处理能力。

### 核心接口

```rust
pub trait EngineExtension: Debug + Send + Sync + EngineCall + EngineDatasource {
    fn name(&self) -> String;                                    // 扩展名称
    fn set(&mut self, key: &str, value: &str) -> Result<...>;   // 设置配置
    fn get(&self, key: &str) -> Result<String, ...>;            // 获取配置  
    fn options(&self) -> Vec<EngineExtensionOption>;             // 列出所有配置项
}
```

`EngineDatasource`接口负责向Probing的数据处理核心`DataFusion`输送数据，而`EngineCall`接口则赋予Probing服务器额外的远程API调用能力。这套设计精巧的接口共同构成了Probing扩展系统的基石。

### 数据源核心接口 - EngineDatasource

Probing的数据处理能力建立在一个灵活的数据源抽象之上。当我们谈论性能诊断和监控时，数据收集是一切分析的基础。`EngineDatasource`接口正是这个数据桥梁的核心所在，它负责将各式各样的数据源接入Probing的查询引擎。

```rust
pub trait EngineDatasource {
    fn datasrc(&self, namespace: &str, name: Option<&str>) 
        -> Option<Arc<dyn Plugin + Sync + Send>>;
}
```

`EngineExtension`通过实现`EngineDatasource`接口向Probing提供数据源插件。在Probing中，所有数据都以`namespace.table_name`的形式进行组织和访问。基于此，数据源插件主要分为两种类型：

- `CustomTable` (静态表格插件): 此类插件提供具有固定结构和内容的数据表，非常适合用于展示系统固有信息、配置项或不经常变动的数据集。
- `CustomNamespace` (动态命名空间插件): 此类插件能够根据查询请求动态地生成表格列表和内容，适用于封装那些数据结构或内容频繁变化的复杂数据源，或是与外部系统交互获取实时数据。

这两种插件类型共同构成了Probing数据访问的骨架。可以将静态表格想象成数据仓库中预先定义好的、结构固定的视图，而动态命名空间则更像一个智能的、按需服务的数据代理，能够灵活连接并转换来自各种外部系统的数据。这种双层设计既保证了在处理简单、静态数据时的易用性与高效性，也为集成复杂、动态的数据源提供了充足的灵活性和强大的功能。

通过这种统一的接口设计，用户可以将各种原本零散、异构的数据源整合到Probing强大的SQL查询体系之中。这意味着开发者不再需要为每一种新的数据格式或数据源编写专门的解析和处理代码。无论是监控CPU使用率这样的系统指标，还是分析网络流量、检查应用日志，甚至是查询自定义的业务数据，都可以通过标准、一致的SQL接口来完成，极大地简化了数据获取与分析的复杂度。

这个看似简单的`EngineDatasource`接口，其背后蕴含着强大的扩展能力。它允许Probing以一种统一和标准化的方式，访问种类繁多的异构数据源——从操作系统内部的性能计数器、应用程序暴露的内部状态信息，到外部数据库的查询结果，甚至是远程微服务的API响应。当一个扩展实现了这个接口时，它实际上是在为Probing庞大的数据宇宙贡献一个新的、可被探索和分析的维度。具体来说，该接口的设计带来了以下关键优势：

- **清晰的命名空间管理**：通过`namespace`参数，Probing能够将来自不同来源、不同类型的数据进行逻辑上的分组与隔离，从而形成层次清晰、易于管理的数据视图。
- **灵活的动态数据发现**：`name`参数的可选性设计，使得`EngineDatasource`接口既能支持对特定数据表的精确查询，也能够支持对整个命名空间下所有可用数据源的动态发现和枚举，这对于探索未知或动态变化的数据环境尤为重要。
- **强大的多态插件系统**：接口的返回值采用Rust语言的特征对象（trait object）设计，这使得不同类型、不同实现的数据源插件可以被无缝地集成到Probing统一的查询框架之中，并被一致地处理。

一个典型的`EngineDatasource`实现，可能会连接到操作系统的性能计数器以收集系统负载信息，或者查询应用程序的内部状态以监控其健康度，也可以是从一个专门的数据库中拉取业务数据，甚至是调用一个远程服务API来获取第三方信息。通过`EngineDatasource`接口的适配与转换，Probing将这些来源各异的数据统一转化为结构化的、可被SQL查询的表格形式，从而使开发者能够运用他们所熟悉的SQL语法，进行复杂的、跨数据源的深度数据分析与洞察。

### 数据源插件

当我们深入了解Probing的数据访问机制，会发现其真正的力量来自于多样化的数据源插件实现。Probing提供了两种核心插件类型，各自承担不同的数据访问职责。

**静态表格插件（TablePlugin）：**

静态表格插件是Probing扩展系统中最直观的数据提供者。它们就像是预先定义好的数据视图，具有固定的结构和内容，非常适合展示系统配置、静态信息或缓存数据。

```rust
pub trait CustomTable {
    fn name() -> &'static str;      // 表名
    fn schema() -> SchemaRef;       // 表结构 
    fn data() -> Vec<RecordBatch>;  // 数据批次
}

// 注册插件
let plugin = TablePluginHelper::<MyTable>::create("namespace", "table_name");
```

实现这个特性的插件能够以最小的代码量提供数据访问能力。开发者只需定义表名、数据结构和内容，Probing就会自动处理SQL查询解析、数据过滤和结果返回等操作。这种简洁性使得快速开发特定用途的数据表变得非常容易。

在性能诊断场景中，静态表格可用于展示系统基础信息，如CPU核心数、内存配置、运行时环境变量或编译选项等。这些信息虽然简单，但对于理解系统整体状态至关重要。

**动态表格插件（SchemaPlugin）：**
```rust
#[async_trait]
pub trait CustomNamespace {
    async fn table_names(&self) -> Result<Vec<String>>;
    async fn table(&self, name: &str) -> Result<Arc<dyn TableProvider>>;
}
```

与静态表格不同，动态命名空间插件提供了更强大的灵活性。它可以根据查询请求动态生成表格列表和内容，实现与外部数据源的实时交互。这种设计非常适合处理变化的数据集，例如实时监控指标、日志流或远程API响应。

动态命名空间的核心优势在于其自适应能力。`table_names`方法允许插件根据当前环境动态发现可用数据表，而`table`方法则负责按需创建特定表格的查询接口。这种延迟加载策略不仅提高了资源利用效率，还使得插件能够适应复杂多变的数据源环境。

在实际应用中，一个动态命名空间插件可能会连接到操作系统的进程列表，并为每个进程动态创建独立的数据表，展示其资源使用情况、打开的文件句柄或内存映射等信息。这种动态生成的表格集合为开发者提供了丰富而精准的系统视图。

### API接口插件

除了数据源插件外，Probing还提供了另一个强大的扩展点：API接口。通过`EngineCall`特性，扩展可以向Probing服务器添加自定义的HTTP端点，实现更复杂的交互逻辑和命令执行能力。

```rust
#[async_trait]
pub trait EngineCall {
    async fn call(
        &self,
        path: &str,                           // API路径
        params: &HashMap<String, String>,     // 查询参数
        body: &[u8],                         // 请求体
    ) -> Result<Vec<u8>, EngineError>;       // 响应数据
}
```

这个接口设计简洁而灵活，类似于一个微型的Web服务框架。扩展可以通过实现这个特性来处理来自客户端的各种请求：

- 执行动态生成的代码片段
- 注入诊断逻辑到目标进程
- 提供交互式调试控制
- 实时修改监控参数
- 导出分析结果到外部格式

API接口和数据源的结合形成了Probing的完整能力闭环：数据源提供了观测系统状态的"眼睛"，而API接口则是干预系统行为的"手"。通过这两种能力的协同，扩展可以实现从问题发现到故障诊断再到动态修复的完整工作流。

**现有Rust扩展示例：**
- **C/C++扩展**：原生代码调试、内存分析、系统调用追踪
- **Python扩展**：REPL、火焰图、调用栈追踪、对象检查
- **Torch扩展**：PyTorch模型监控和性能分析

### 配置管理

```rust
// 扩展配置项定义
pub struct EngineExtensionOption {
    pub key: String,           // 配置键
    pub value: Option<String>, // 当前值
    pub help: &'static str,    // 帮助信息
}

// 运行时配置
manager.set_option("my_option", "new_value")?;
let value = manager.get_option("my_option")?;
```

## 扩展开发指南

### Python扩展开发

1. **轻度扩展**：直接编写Python函数，在SQL中调用
2. **重度扩展**：使用`@table`装饰器创建数据表，支持持久化
3. **测试集成**：确保数据类型转换正确，SQL查询正常

### Rust扩展开发

1. **实现核心trait**：根据需求实现`EngineCall`和/或`EngineDatasource`
2. **定义配置项**：在`options()`方法中列出所有可配置参数
3. **注册插件**：通过`EngineExtensionManager`注册到系统
4. **性能优化**：利用Arrow列式存储和DataFusion查询优化

扩展系统与DataFusion查询引擎深度集成，支持标准SQL语法访问所有扩展提供的数据源。
