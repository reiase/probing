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
probing -t <pid> inject [OPTIONS]
```

选项：`-P,--pprof` 启用 profiling；`-c,--crash` 启用崩溃处理；`-l,--listen <ADDRESS>` 在指定地址服务监听远程连接。

2. **通过环境变量来注入**

通过环境变量，可以快速为Python进程启用`probing`

``` bash
PROBING=1 python script.py
```

PROBE 环境变量作为激活和配置探针功能的主要机制，支持以下值和行为：

| Value              | Behavior                                     |
| ------------------ | -------------------------------------------- |
| `1` or `followed`  | 仅为当前进程启用探针功能                     |
| `2` or `nested`    | 为当前进程及其所有子进程启用探针功能         |
| `<script_name>.py` | 仅为指定脚本名称的进程启用探针功能           |
| `regex:<pattern>`  | 仅为匹配指定正则表达式模式的进程启用探针功能 |

### 命令行

`probing`通过一系列指令控制探针来获取数据或是执行特定操作，以下为`probing`的命令行：

```
Probing CLI - A performance and stability diagnostic tool for AI applications

Usage: probing [OPTIONS] [TARGET] [COMMAND]

Commands:
  inject     Inject into the target process [aliases: in, i]
  config     Display or modify the configuration
  backtrace  Show the backtrace of the target process or thread [aliases: bt]
  eval       Evaluate Python code in the target process
  query      Query data from the target process
  launch     Launch new Python process
  help       Print this message or the help of the given subcommand(s)

Arguments:
  [TARGET]  target process, PID (e.g., 1234) for local process, and <ip>:<port> for remote process

Options:
  -v, --verbose  Enable verbose mode
  -h, --help     Print help
  -V, --version  Print version

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
