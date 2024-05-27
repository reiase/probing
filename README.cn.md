# Probe: AI应用的性能与稳定性诊断工具

Probe 是面向AI应用设计的性能与稳定性诊断工具，旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）难以调试与调优的问题。通过向目标进程植入探针服务(probe server) ，可以实现更为详细的性能数据采集，或是实时修改目标进程的执行行为。

## 主要特性

Probe的主要功能是向目标进程注入探针，并借助探针实现问题调试与性能诊断功能，具体包含：

- 调试功能：打印调用堆栈、提供REPL交互等；
- 性能剖析：性能采样与输出火焰图；
- 进阶功能：向进程注入任意代码、支持远程调试等；

相比其他调试与诊断工具，探针方式具备如下特点：

1. **无代码侵入**: 无需修改代码即可实现Instrumentation，实现函数调用跟踪、性能数据采集等功能；

2. **无环境依赖**: 每个进程内建独立的数据采集和存储，无需部署复杂的分布式数据采集与存储系统即可直接使用；

3. **低性能开销**：性能数据采集与问题诊断通过旁路实现无需埋点，极大地减少了对目标进程的性能影响；

4. **即插即用**：可在任意时刻侵入目标进程进行诊断，无需中断或重启。特别适用于 LLM 训练等长周期任务；

## Quick Start

### 注入探针

```shell
probe <pid> inject [OPTIONS]
```

选项：--pprof 启用 profiling；--crash 启用崩溃处理；--background 启用后台服务；--address <ADDRESS> 指定服务监听地址。

### 诊断问题

注入探针后可以借助probe提供命令进行问题诊断：

- `dump`命令：打印当前调用栈，用于定位进程阻塞和死锁问题；

    ```sh
    probe <pid> dump
    ```

- `pause`命令：暂停进程并启动远程调试服务;

    ```sh
    probe <pid> pause [ADDRESS] # ADDRESS参数可选，默认为随机端口
    nc 127.0.0.1 3344           # 使用nc连接调试服务
    ```

- `catch`命令：接管错误处理，在出错时启动远程服务;

    ```sh
    probe <pid> catch
    ```

- `listen`命令: 启动后台调试服务:

    ```sh
    probe <pid> listen [ADDRESS] # ADDRESS参数可选，默认为随机端口
    nc 127.0.0.1 3344            # 使用nc连接调试服务
    ```

- `execute`命令：注入并执行代码；

    ```sh
    probe <pid> execute <SCRIPT> # 
    # 比如
    probe <pid> execute script.py 
    probe <pid> execute "import traceback;traceback.print_stack()"
    ```

- `pprof`命令: 启动profiling;

    ```sh
    probe <pid> pprof

    # 等待一段时间后获取火焰图
    sleep 10
    curl http://127.0.0.1:3344/flamegraph > flamegraph.svg
    ```
