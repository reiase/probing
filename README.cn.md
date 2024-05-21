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

可以通过环境变量控制probe server的行为

```bash
export PROBE_PPROF=1 
export PROBE_ADDR=127.0.0.1:3344 
export PROBE_BG=1 
LD_PRELOAD=<prefix_path>/libprobe.so python a.py
```

#### `ptrace` 注入
probe提供命令行工具，通过`ptrace`系统调用向目标进程注入`probe server`
```bash
<prefix path>/probe  [--dll <DLL路径>] <目标进程PID>
```
如果未指定 --dll 参数，工具将默认使用当前可执行文件路径下的 libprobe.so。

#### 连接`probe server`

完成`probe server`注入后，可通过网络连接与使用。`probe server`支持两种连接协议：
- 纯文本协议：可以通过netcat命令与`probe server`交互`nc 127.0.0.1 3344`;
- `HTTP`协议：可以通过浏览器访问`probe server`，获取相关信息`http://127.0.0.1:3344/`；

# 问题诊断

probe可以帮助用户诊断和定位进行住问题，已交互或非交互的方式提供信息：

1. 打印python调用栈：
```bash
$ probe --dump <pid> 
```
目标进程将会打印Python代码的调用栈，方便用户定位进程hang在何处。

2. 启动临时调试:
```bash
$ probe --pause <pid>
```
目标进程将会暂停执行，并在调用堆栈上启动调试服务器，用户可以连接服务器来交互式分析Python的调用栈。

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