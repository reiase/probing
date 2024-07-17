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

### 探针注入

`probing`通过探针采集数据和控制目标进程，有两种方式用于注入探针：

1. **通过命令行注入**

```shell
probing <pid> inject [OPTIONS]
```

选项：`-P,--pprof` 启用 profiling；`-c,--crash` 启用崩溃处理；`-l,--listen <ADDRESS>` 在指定地址服务监听远程连接。

2. **通过代码注入**

```python
import probing
probing.init(listen="127.0.0.1:9922")
```

### 命令行与REPL

`probing`通过一系列指令控制探针来获取数据或是执行特定操作，以下为`probing`的命令行：

```
Probing CLI - A performance and stability diagnostic tool for AI applications

Usage: probing [OPTIONS] <TARGET> [COMMAND]

Commands:
  inject     Inject into the target process [aliases: inj, i]
  panel      Interactive visualizer in terminal [aliases: pnl, console]
  repl       Repl debugging shell
  enable     Enable features (`-h, --help` to see full feature list)
  disable    Disable features (see `-h, --help` above)
  show       Display informations from the target process (see `-h, --help` above)
  backtrace  Show the backtrace of the target process or thread [aliases: bt]
  eval       Evaluate code in the target process
  help       Print this message or the help of the given subcommand(s)

Arguments:
  <TARGET>  target process, PID (e.g., 1234) or `Name` (e.g., "chrome.exe") for local process, and <ip>:<port> for remote process

Options:
  -v, --verbose  Enable verbose mode
      --ptrace   Send ctrl commands via ptrace
  -h, --help     Print help

```

其中`enable`，`disable`，`show`，`backtrace`和`eval`是主要的控制指令：
- enable：启用某特性，特性列表如下：
  - pprof：启用profinling；
  - dap：启用dap远程调试；
  - remote：启用tcp远程控制；
  - catch-crash：启用crash handler
- disable：禁用某特性，特性列表同上；
- show：显示目标进程信息
  - memory：内存信息
  - threads：线程信息  
  - objects：python对象信息
  - tensors：pytorch tensor信息
  - modules：pytorch module信息
  - plt：过程链接表（PLT, Procedure Linkage Table）
- backtrace：抓取目标进程调用堆栈
- eval：向目标进程注入特定代码并执行；

上述指令可以通过命令行发送，也可以通过发送。

### Web Panel 与 Console Panel

`probing`的功能可以通过web方式可视化访问，例如：

```shell
probing <pid> inject -l 127.0.0.1:1234
```

之后可以通过浏览器打开`http://127.0.0.1:1234`来使用上述功能。若无法通过浏览器访问，也可从终端打开交互界面：

```shell
probing <pid> panel
```

## 安装probing

### 二进制安装

`probing` 可以通过pip命令安装：

```sh
$pip install probing
```

### 源码构建

`probing` 构建时依赖`trunk`工具，可通过如下命令安装，若已经安装可以跳过此步：
```shell
cargo install trunk
```
构建环境准备就绪后，可以通过`make`命令来完成构建
```shell
$make
```

### 开发模式

为了便于用户使用，probing将python脚本与web app打包进libprobing.so中。开发时每次修改代码都要重新打包会极大的降低效率。
因此这里推荐手动构建:

```shell
# 持续构建web app
cd app
trunk watch --filehash false -d dist/

# 构建probing与libprobing
cargo b -p probing-cli
cargo b
```

在debug模式下，`probing`会自动从dist目录加载web app，从src/加载python脚本，而无需重新打包。
