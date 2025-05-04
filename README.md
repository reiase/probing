# Probing: AI应用的性能与稳定性诊断工具

Probing 是一款专为AI应用设计的性能与稳定性诊断工具，旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）中的调试与优化难题。通过向目标进程植入探针，可以更详细地采集性能数据，或实时修改目标进程的执行行为。

## 主要特性

Probing的主要功能包括：

- 调试功能：
  - 观测目标进程的调用栈、Python对象、Torch Tensor与模块等；
  - 支持远程调试，可通过DAP协议使用VSCode远程调试目标进程；
- 性能剖析：
  - 对C/C++代码进行性能采样，并生成火焰图；
  - 支持Torch的profiling功能，分析模型性能；
- 远程控制：
  - 提供HTTP接口，用于获取数据和控制目标进程执行；
  - 支持远程注入任意Python代码至目标进程。

相比其他调试与诊断工具，`probing`能够即插即用，可在任意时刻侵入目标进程，无需中断或重启，也无需修改代码。

## Quick Start

### 安装

`probing` 可以通过`pip`命令来安装 

``` bash
pip install probing
```

### 探针注入

`probing`通过探针采集数据和控制目标进程，有两种方式用于注入探针：

1. **通过命令行注入**

```shell
probing <pid> inject [OPTIONS]
```

选项：`-P,--pprof` 启用 profiling；`-c,--crash` 启用崩溃处理；`-l,--listen <ADDRESS>` 在指定地址服务监听远程连接。

2. **通过环境变量来注入**

通过环境变量，可以快速为Python进程启用`probing`

``` bash
PROBE=1 python script.py
```

PROBE 环境变量作为激活和配置探针功能的主要机制，支持以下值和行为：

| Value              | Behavior                                     |
| ------------------ | -------------------------------------------- |
| `1` or `followed`  | 仅为当前进程启用探针功能                     |
| `2` or `nested`    | 为当前进程及其所有子进程启用探针功能         |
| `<script_name>.py` | 仅为指定脚本名称的进程启用探针功能           |
| `regex:<pattern>`  | 仅为匹配指定正则表达式模式的进程启用探针功能 |

### 命令行

`probing` 提供了一个命令行界面（CLI）来与探针交互、控制目标进程以及查询数据。

**基本用法:**

```
probing [OPTIONS] [TARGET] [QUERY_STRING] [COMMAND]
```

**参数说明:**

*   `OPTIONS`:
    *   `-v, --verbose`: 启用详细输出模式。
*   `TARGET`: 目标进程。可以是本地进程的 PID (例如 `1234`)，或者是远程进程的地址 (例如 `192.168.1.100:9988`)。如果省略，某些命令可能无法执行或需要其他方式指定目标。
*   `QUERY_STRING`: 一个可选的查询字符串，作为 `query` 命令的快捷方式。例如 `probing <pid> "select * from some_table"` 等同于 `probing <pid> query "select * from some_table"`。
*   `COMMAND`: 要执行的具体子命令。

**主要命令:**

*   `inject [-D <KEY=VALUE>...]`: 向目标进程注入探针。可以使用 `-D` 或 `--define` 来设置探针配置项（例如 `-D probing.log_level=debug`）。如果探针已注入，此命令可用于更新配置。
*   `list [-v] [-t]`: 列出所有已注入探针的进程。
    *   `-v, --verbose`: 显示更详细的信息（包括探针通信socket）。
    *   `-t, --tree`: 以树状结构显示进程关系。
*   `config [SETTING]`: 显示或修改目标进程中的探针配置。
    *   不带参数: 显示所有 `probing.` 开头的配置项。
    *   带参数 (例如 `probing.log_level=info`): 设置指定的配置项。
*   `backtrace [TID]`: 显示目标进程或指定线程 (`TID`) 的调用栈。
*   `eval <CODE>`: 在目标进程中执行指定的 Python 代码片段。
*   `query <QUERY>`: 向目标进程发送查询语句（类似 SQL）并获取数据。
*   `launch [-r] <ARGS...>`: 启动一个新的 Python 进程，并自动注入探针。
    *   `-r, --recursive`: 同时为启动的进程及其所有子进程注入探针。
    *   `<ARGS...>`: 要执行的命令及其参数 (例如 `python my_script.py --arg1 value1`)。

**示例:**

```bash
# 注入探针到 PID 为 1234 的进程，并设置日志级别
probing 1234 inject -D probing.log_level=debug

# 列出所有被探测的进程（树状视图）
probing list -t

# 查询 PID 为 1234 进程中的某个表
probing 1234 query "select * from torch_modules"

# 获取 PID 为 1234 进程的配置
probing 1234 config

# 设置 PID 为 1234 进程的采样间隔
probing 1234 config probing.sample_interval_ms=50

# 查看 PID 为 1234 进程的主线程调用栈
probing 1234 backtrace

# 在 PID 为 1234 的进程中执行 Python 代码
probing 1234 eval "print(1 + 2)"

# 启动一个脚本并自动注入探针（包括子进程）
probing launch -r python train.py --epochs 10
```

## 开发与构建

### 二进制安装

`probing` 可以通过pip命令安装：

```sh
$pip install probing
```

### 源码构建

`probing`可以通过cargo来进行构建

```bash
cargo build                 #构建so文件
cargo build -p probing-cli  #构建probing命令行
```

python包通过`make_wheel.py`脚本来构建
```bash
python make_wheel.py
```

构建可发布的wheel包：
```bash
make ZIG=1
```

`ZIG=1`将会启用`cargo zigbuild`，通过`zig`语言的工具链进行连接，提供更好glibc的兼容性（解决了不同Linux发行版之间的glibc版本差异问题，允许在较新系统构建的二进制在较旧系统上运行）。
