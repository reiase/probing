# Probe: AI应用的性能与稳定性诊断工具

Probe 是面向AI应用设计的性能与稳定性诊断工具，旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）难以调试与调优的问题。通过向目标进程植入探针服务(probe server) ，可以实现更为详细的性能数据采集，或是实时修改目标进程的执行行为。

## 主要特性

1. **无代码侵入**: 无需修改代码即可实现Instrumentation，实现函数调用跟踪、性能数据采集等功能；

2. **无环境依赖**: 每个进程内建独立的数据采集和存储，无需部署复杂的分布式数据采集与存储系统即可直接使用；

3. **低性能开销**：性能数据采集与问题诊断通过旁路实现无需埋点，极大地减少了对目标进程的性能影响；

4. **即插即用**：可在任意时刻侵入目标进程进行诊断，无需中断或重启。特别适用于 LLM 训练等长周期任务；

# Quick Start

probe通过向目标进程注入`probe server`实现其功能，`probe server`的注入方式有两种：进程启动时通过`LD_PRELOAD`注入与进程启动后通过`ptrace`系统调用注入。

#### `LD_PRELOAD` 注入

```bash
LD_PRELOAD=<prefix_path>/libprobe.so python a.py 
```

#### `ptrace` 注入
probe提供命令行工具，通过`ptrace`系统调用向目标进程注入`probe server`
```bash
<prefix path>/probe  [--dll <DLL路径>] <目标进程PID>
```
如果未指定 --dll 参数，工具将默认使用当前可执行文件路径下的 libprobe.so。

# 使用Probe诊断问题

在注入`probe server`之后，可以借助`probe`命令对目标进行进行操作：

```
Usage: probe [OPTIONS] <PID>

Arguments:
  <PID>  target process

Options:
      --dll <DLL>          dll file to be injected into the target process, default: <location of probe cli>/libprobe.so
  -d, --dump               signal libprobe to dump the calling stack of the target process
  -p, --pause              signal libprobe to pause the target process and listen for remote connection
  -P, --pprof              signal libprobe to start profiling
  -c, --crash              signal libprobe to handle target process crash
  -b, --background         signal libprobe to start background server
  -e, --execute <EXECUTE>  signal libprobe to execute a script in the target process
  -a, --address <ADDRESS>  address used for listening remote connection
  -t, --test               
  -h, --help               Print help
```

### `-d, --dump`: 打印当前运行堆栈

目标进程打印当前Python运行堆栈信息，并继续执行。可以用于定位长时间没有响应的进程的执行状态。

### `-p, --pause [-a|--address <ADDRESS>]`: 暂停进程并启动远程服务

暂停目标进程，并在当前栈上启动远程服务，`-a, --address <ADDRESS>`参数用于指定服务地址。服务启动后可使用`netcat`命令链接，进入一个Python解释器交互界面：

```shell
nc 127.0.0.1 3344
```

### `-b, --background [-a|--address <ADDRESS>]`: 启动后台调试服务

在目标进程中开启新线程执行远程服务,`-a, --address <ADDRESS>`参数用于指定服务地址。服务启动后连接与交互方式同上。

### `-e, --execute <EXECUTE>`: 执行注入代码

将`<EXECUTE>`指定的代码注入目标进程，并立刻执行。`<EXECUTE>`可以是文件名或者代码片段。比如：
```
probe -e script.py <pid>
probe -e "import traceback;traceback.print_stack()" <pid>
```

进入该界面后可通过Python语句与进程自身的解释器交互。

### `-c, --crash`: 接管错误处理

probe将会接管目标进程的异常信号处理（比如`SIGABRT`）。当发代码运行生异常时，会启动远程服务，可以远程链接并调试；

### `-P, --pprof`: 启动profiling

目标进程将会自动采样运行堆栈，配合`-b, --background`所启动的后台服务，可以通过HTTP接口读取火焰图。

# 设计思考

Probe主要基于Rust语言开发，并且使用基于rust的Python解释器RustPython作为脚本语言。

## 关于Rust语言

Rust语言提供与C语言类似的与底层交互能力，同时又引入了很多现代化特性，而不受ABI问题困扰：
1. C语言互操作性，Rust 能够很好的与C语言构建的底层系统进行交互，额无需担心ABI兼容性问题；
2. 默认静态链接，能够很大程度避免复杂的依赖库版本问题；
3. 依赖管理，Cargo下无需额外工作，即可管理好第三方库的版本与构建，虽然CMake也有第三方提供依赖管理，但总是难以做到开箱即用；

Probe中借助Rust语言的特性，能够为一款底层工具添加很多易用的高阶用户交互特性，包括网络访问、http接口以及flamegraph等特性。
如果直接基于C/C++开发，将会不得已去处理大量三方依赖问题，甚至自己去重写某些重要特性。

## 关于RustPython

Python是一门普及非常广的语言，非常适合作为扩展工具用的脚本语言。然而直接使用系统Python将会面临诸多问题：
1. Python版本兼容性问题：系统中可能同时安装了多个版本的Python解释器，Probe作为一款底层工具，很难确定链接哪一个解释器；
2. Probe的Python代码可能会与用户自己的Python代码冲突；

因此Probe使用RustPython作为嵌入的Python解释器，一方面不会产生链接问题，另一方面也隔离了用户代码与Probe自身的Python代码。