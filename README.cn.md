# Probe 一款超长周期问题诊断工具

随LLM兴起的超大规模异构计算任务开始变得越来越多，然而这些异构计算任务通常受稳定性与性能等多方面问题困扰。
而传统的稳定性与性能分析工具在处理这类问题时往往力不从心：
1. 异构计算，GPU或者加速器的性能与问题同样需要关注，因此gperftools等CPU侧工具往往力不从心；
2. 超大规模集群，使得单机性能工具在部署和使用不够友好，特别是coredump等工具在保留现场时还需要解决core文件存储问题；
3. 海量数据，使得grafana等传统的微服务监控软件，难以承担海量数据的传输与存储负担。

Probe尝试面向超大规模异构计算问题给出性能与问题定位的解决方案：
1. 分布式采集和存储：每个进程都在进程内自己管理性能数据的采集和存储；
2. 低侵入性：通过LD_PRELOAD注入目标进程，而无需修改目标进程的源码；
3. 低开销：通过信号量机制触发执行，对目标进程的影响可忽略；

# Quick Start

```bash
LD_PRELOAD=target/release/libprobe.so python a.py 
```

可以通过环境变量控制probe的行为

```bash
export PROBE_PPROF=1 
export PROBE_ADDR=127.0.0.1:3344 
export PROBE_BG=1 
LD_PRELOAD=target/release/libpguard.so python a.py
```

之后可以通过netcat命令连接到probe开启的调试后门

```bash
nc 127.0.0.1 3344
```

也可以通过浏览器打开probe的web页面:`http://127.0.0.1:3344/`

# 一些设计思考

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