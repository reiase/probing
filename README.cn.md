# Probe: AI应用的性能与稳定性诊断工具

Probe 是面向AI应用设计的性能与稳定性诊断工具，旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）难以调试与调优的问题。通过向目标进程植入探针服务(probe server) ，可以实现更为详细的性能数据采集，或是实时修改目标进程的执行行为。

## 主要特性

Probe的主要功能是向目标进程注入探针，并借助探针实现问题调试与性能诊断功能，具体包含：

- 调试功能：
  - 观测目标进程call stack、Python对象、Torch Tensor与Module等；
  - 远程调试，可借助DAP协议通过vscode远程调试目标进程；
- 性能剖析：
  - C/C++代码的性能采样，并输出火焰图；
  - Torch的profiling功能，分析模型的性能；
- 远程控制：
  - 提供http接口来获取数据、控制目标进程执行；
  - 通过远程控制向目标进程注入任意Pyhton代码；

相比其他调试与诊断工具，`probe`能够即插即用，可在任意时刻侵入目标进程，无需中断或重启，也无需修改代码。

## Quick Start

### 注入探针

```shell
probe <pid> inject [OPTIONS]
```

选项：`-P,--pprof` 启用 profiling；`-c,--crash` 启用崩溃处理；`-b,--background` 启用后台服务；`-a,--address <ADDRESS>` 指定服务监听地址。

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

### 进阶功能

probe 为大模型的开发与调试提供了一系列Python分析与诊断功能：

- Activity分析：可以抓取每个线程当前执行的Python堆栈信息；
- Debug功能：启动Python远程调试功能，可以在vscode中调试目标进程；
- Profile功能：对torch模型的执行进行profiling；
- Inspect功能：用于检视Python对象、torch Tensor对象与torch Module模型；

这些功能都可以通过web界面来访问，比如注入探针时指定服务地址：
```shell
probe <pid> inject -b -a 127.0.0.1:1234
```
之后可以通过浏览器打开`http://127.0.0.1:1234`来使用上述功能。
