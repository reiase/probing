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