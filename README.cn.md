# Probing: AI应用的性能与稳定性诊断工具

Probing 是一款专为AI应用设计的性能与稳定性诊断工具，旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）中的调试与优化难题。通过向目标进程植入探针服务（probing server），可以更详细地采集性能数据，或实时修改目标进程的执行行为。

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

**通过命令行注入**

```shell
probing --pid <pid> inject [OPTIONS]
```

选项：`-P,--pprof` 启用 profiling；`-c,--crash` 启用崩溃处理；`-l,--listen <ADDRESS>` 在指定地址服务监听远程连接。

**通过代码注入**

```python
import probing
probing.init(listen="127.0.0.1:9922")
```

### 调试与性能诊断

注入探针后，可以使用probing提供的命令进行问题诊断：

- `debug`命令（别名`dbg`或`d`），调试与检查工具，用于定位进程阻塞和死锁问题；

    ```sh
    $ probing help debug
    Debug and Inspection Tool

    Usage: probing debug [OPTIONS]

    Options:
      -d, --dump               Dump the calling stack of the target process
      -p, --pause              Pause the target process and listen for remote connection
      -a, --address <ADDRESS>  address to listen [default: 127.0.0.1:9922]
      -h, --help               Print help
    ```

    例如：

    ```sh
    $probing -p <pid> debug --dump # 打印目标进程的当前调用堆栈
    $probing -p <pid> d -d         # 同上，使用简化命令

    $probing -p <pid> debug --pause --address 127.0.0.1:9922 #暂停目标进程，并等待远程连接
    $probing -p <pid> d -p -a 127.0.0.1:9922                 # 同上，使用简化命令
    ```

- `performance`命令（别名：`perf`或`p`）：性能诊断工具，用于收集性能数据、诊断性能瓶颈;

    ```sh
    $probing help performance
    Performance Diagnosis Tool

    Usage: probing performance [OPTIONS]

    Options:
          --cc     profiling c/c++ codes
          --torch  profiling torch models
      -h, --help   Print help
    ```

    例如：

    ```sh
    $probing -p <pid> perf --cc    # 启用c/c++ 的profiling，可输出flamegraph
    $probing -p <pid> perf --torch # 启用torch的profiling
    ```

### 进阶功能

probing 为大模型的开发与调试提供了一系列Python分析与诊断功能：

- Activity分析：捕获每个线程当前执行的Python堆栈信息；
- Debug功能：启动Python远程调试功能，可在VSCode中调试目标进程；
- Profile功能：对torch模型执行进行profiling；
- Inspect功能：用于检视Python对象、torch Tensor对象与torch Module模型；

这些功能可以通过web界面访问。注入探针时指定服务地址，例如：

```shell
probing <pid> inject -l 127.0.0.1:1234
```

之后可以通过浏览器打开`http://127.0.0.1:1234`来使用上述功能。

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
