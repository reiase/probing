# Probe: AI应用的性能与稳定性诊断工具

Probe 是一款专为AI应用设计的性能与稳定性诊断工具，旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）中的调试与优化难题。通过向目标进程植入探针服务（probe server），可以更详细地采集性能数据，或实时修改目标进程的执行行为。

## 主要特性

Probe的主要功能包括：

- 调试功能：
  - 观测目标进程的调用栈、Python对象、Torch Tensor与模块等；
  - 支持远程调试，可通过DAP协议使用VSCode远程调试目标进程；
- 性能剖析：
  - 对C/C++代码进行性能采样，并生成火焰图；
  - 支持Torch的profiling功能，分析模型性能；
- 远程控制：
  - 提供HTTP接口，用于获取数据和控制目标进程执行；
  - 支持远程注入任意Python代码至目标进程。

相比其他调试与诊断工具，`probe`能够即插即用，可在任意时刻侵入目标进程，无需中断或重启，也无需修改代码。

## Quick Start

### 注入探针

```shell
probe <pid> inject [OPTIONS]
```

选项：`-P,--pprof` 启用 profiling；`-c,--crash` 启用崩溃处理；`-b,--background` 启用后台服务；`-a,--address <ADDRESS>` 指定服务监听地址。

### 诊断问题

注入探针后，可以使用probe提供的命令进行问题诊断：

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

- Activity分析：捕获每个线程当前执行的Python堆栈信息；
- Debug功能：启动Python远程调试功能，可在VSCode中调试目标进程；
- Profile功能：对torch模型执行进行profiling；
- Inspect功能：用于检视Python对象、torch Tensor对象与torch Module模型；

这些功能可以通过web界面访问。注入探针时指定服务地址，例如：
```shell
probe <pid> inject -b -a 127.0.0.1:1234
```
之后可以通过浏览器打开`http://127.0.0.1:1234`来使用上述功能。

## 安装probe

### 二进制安装
`probe` 无需可以安装，下载release文件后直接解压执行即可。用户可以根据需要自行将probe加入`$PATH` 环境变量。

### 源码构建

`probe` 构建时依赖`trunk`工具，可通过如下命令安装，若已经安装可以跳过此步：
```shell
cargo install trunk
```
构建环境准备就绪后，可以通过`build.sh`脚本来完成构建
```shell
sh build.sh
```

### 开发模式

为了便于用户使用，probe将python脚本与web app打包进libprobe.so中。开发时每次修改代码都要重新打包会极大的降低效率。
因此这里推荐手动构建:

```shell
# 持续构建web app
cd app
trunk watch  --filehash false -d ../dist/

# 构建probe与libprobe
cargo b -p cli
cargo b
```

在debug模式下，`probe`会自动从dist目录加载web app，从src/加载python脚本，而无需重新打包。