<style type="text/css" rel="stylesheet">
body {
    counter-reset: h2
}
h1 {
  counter-reset: h2;
}
h2 {
  counter-reset: h3;
}
h2::before {
  counter-increment: h2;
  content: counter(h2) ". ";
}
h3::before {
  counter-increment: h3;
  content: counter(h2) "." counter(h3) ". ";
}
</style>

本文详细介绍Probing的性能分析实现。关于Probing的整体架构设计，请参考[整体架构](./arch.md)。

在进行系统优化时，Benchmark与Profiling是非常重要的两种手段。前者是指测试整个系统端到端性能，或者系统某个子模块的端到端性能的方法，可以非常快速的形成：代码优化 --> Benchmark --> 代码优化 这样一个带有反馈的循环。也是大多数进行性能优化的工程师最主要的工作流。然而单纯的Benckmark工作流非常容易陷入拼命投入优化代码，但性能始终不见提升的情况。此时所投入的优化已经不再是整个系统的核心瓶颈了，因此无法为整个系统的性能带来太多提升。如何定位系统的性能问题，找到需要优化的瓶颈就需要进行Profiling。

## Profiling方法论

日常工作中，大家能够到很多profiling工具，比如Linux内核自带的perf、intel的vtune以及torch的torch.profiler等。这些工具根据底层原理的不同，可分为三类：插桩（Instrumentation）、采样（Sampling）和性能计数器（Peformance Counter）。

### Instrumentation方法

在测试代码性能时，经常用到的方法就是在代码里直接插入计时代码：

```python
def foo():
  start = time.time()
  do something
  end = time.time()

  print("duration = ", end - start)
```

除了记录时间以外，这种手动插入代码还能记录分支和循环的情况、内存分配的情况等分析性能时感兴趣的信息。如果能够将这种过程自动化，为所执行的每一段代码都插入相关计时，我们就能从全局视角对整个应用进行Profiling了，这也是自动Instrumentation技术。对于C/C++代码，这种插桩需要编译器配合，比如GCC有profiling相关选项：
```bash 
gcc a.c -g -pg
```
借助Instrumentation，编译器还能实现代码覆盖率、AddressSanitizer、StackProtector等功能。除了直接在编译阶段影响二进制代码的生成，还可以在linker/loader中做手脚，比如大多数memory profiler都是通过替换libc中的malloc/free函数来实现对内存分配行为的追踪。

#### Python代码的插桩

对于Python来说，最常见的插桩方法是使用Decorator：

```python
def my_decorator(func):
  def wrapper(*args, **kwargs):
    start = time.time()
    ret = func(*args, **kwargs)
    end = time.time()
    print("duration of {}: {}", func, end - start)
    return ret
  return wrapper
```
使用`my_decorator`时：
```python
@my_decorator
def foo():
  pass
```
会自动被展开为：
```python
def foo():
  pass
foo = my_decorator(foo)
```
Python还支持带参数的decorator，这里不进一步展开。decorator语法极大简化了Python中插桩的开发成本，并且易于在代码中进行控制。与decorator语法相配合的是`with`语法，该语法定义了一个语义scope，可以在借助这个scope控制插桩代码的逻辑：

```python
def my_decorator(func):
  def wrapper(*args, **kwargs):
    enabled = get_enable()
    if enabled:
      start = time.time()
    ret = func(*args, **kwargs)
    if enabled:
      end = time.time()
      print("duration of {}: {}", func, end - start)
    return ret
  return wrapper

class MyContext:
  def __enter__(self):
    set_enable()
  def __exit__(self):
    set_disable()
```

此时可以借助`MyContext`控制插桩代码的行为：
```python
foo()   # 不会执行插桩代码
with MyContext():
  foo() # 会打印插桩计时

foo()   # 不会执行插桩代码
```

#### Python解释器插桩

除了在代码中进行插桩操作以外，Python还支持在解释器层面进行插桩操作。Python解释器支持用户自定义trace函数：

```C
struct _frame {
    PyObject_HEAD
    PyFrameObject *f_back;      /* previous frame, or NULL */
    struct _PyInterpreterFrame *f_frame; /* points to the frame data */
==> PyObject *f_trace;          /* Trace function */
    int f_lineno;               /* Current line number. Only valid if non-zero */
    char f_trace_lines;         /* Emit per-line trace events? */
    char f_trace_opcodes;       /* Emit per-opcode trace events? */
    PyObject *f_extra_locals;   /* Dict for locals set by users using f_locals, could be NULL */
    ...
};
```

在执行Python代码时，可以借助这个`f_trace`函数来trace解释器内部的执行，共有5种事件会被转发给trace函数：
- call：执行函数调用；
- line：执行一行代码；
- return：函数返回；
- exception：发生一场；
- opcode：执行一条字节码；
借助`trace`函数可以实现Python的调试器，profiler等。如果我们需要实现一个分布式调试器，可以开发一个受RPC控制的trace函数。

#### Torch框架插桩

Torch框架的dispatch机制中预留了一些hook接口，可以通过`torch::addGlobalCallback`接口来捕获算子调用。这些hook会被传入一个`torch::RecordFunction`结构体，通过这个结构体可以获取调用上下文，包括name，inputs等。但是记录这些调用信息的开销很高，需要做好控制。同时Torch也提供`c10::ThreadLocalDebugInfo`接口，用于在整个forward和backward过程中追踪一些信息。

### Sampling方法

插桩方法能够获得准确的执行时间线，输出timeline。但是插桩需要侵入目标代码，并且会带来额外的运行时开销，难以直接用于线上生产任务。

Sampling方法可以很好的避免对目标代码的侵入，同时也能把性能开销控制在一个合理的水平上。常见的Sampling方法有两种：

#### Pprof方法

Pprof方法最先见于google的gperftools，其实现原理在于基于`setitimer`方法[^setitimer]设置一个定时器，`setitimer`方法的原型如下：

```C
int setitimer(int which, const struct itimerval *new_value,
              struct itimerval *old_value);
```
其中which有三种取值：

- **ITIMER_REAL** 计时完成时，触发**SIGALRM**信号;
- **ITIMER_VIRTUAL** 计时完成时，触发**SIGALRM**信号，但只在进程活跃时计时;
- **ITIMER_PROF** 计时完成时，触发**SIGPROF**信号；

**SIGPROF**信号会随机选中一个线程，中断其执行，并在其执行堆栈上运行该信号的handler。一般来说，占用CPU越多的线程被选中的概率越高。在该信号的handler中，可以通过backtrace获取被中断线程的调用堆栈，通过libunwind对堆栈进行分析后，可以完成一次采样。综合多次采样结果，可以绘制当前进程的火焰图。

#### PMU方法PMU

Linux提供perf相关接口[^9]，向开发者暴露硬件PMU（Performance Monitoring Unit）的采样能力：

```C
#include <linux/perf_event.h>    /* Definition of PERF_* constants */
#include <linux/hw_breakpoint.h> /* Definition of HW_* constants */
#include <sys/syscall.h>         /* Definition of SYS_* constants */
#include <unistd.h>

int syscall(SYS_perf_event_open, struct perf_event_attr *attr,
            pid_t pid, int cpu, int group_fd, unsigned long flags);
```
由于glibc未提供封装，需要直接通过syscall直接进行系统调用。`perf_event_attr`用来控制PMU采样：
```C
struct perf_event_attr {
    __u32 type;                 /* Type of event */
    __u32 size;                 /* Size of attribute structure */
    __u64 config;               /* Type-specific configuration */

    union {
        __u64 sample_period;    /* Period of sampling */
        __u64 sample_freq;      /* Frequency of sampling */
    };

    __u64 sample_type;  /* Specifies values included in sample */
    __u64 read_format;  /* Specifies values returned in read */

    ...
    union {
        __u32 wakeup_events;    /* wakeup every n events */
        __u32 wakeup_watermark; /* bytes before wakeup */
    };
    ...
};
```
关键字段说明：
- type：用来定义事件类型
  - `PERF_TYPE_HARDWARE` 硬件采样
  - `PERF_TYPE_SOFTWARE` 软件采样
- config： 用来配置事件
  - `PERF_COUNT_HW_CPU_CYCLES` 按时钟周期采样
  - `PERF_COUNT_HW_INSTRUCTIONS` 按指令数采样
- sample_type：用来制定采样事件包含的信息
  - `PERF_SAMPLE_IP` 记录IP指针
  - `PERF_SAMPLE_TIME` 记录时间戳
  - `PERF_SAMPLE_STACK_USER` 记录用户态调用栈
  - `PERF_SAMPLE_CALLCHAIN` 记录调用栈

相比`setitimer`方法，PMU方法能够提供更高的计时精度，并且采样完全由硬件完成，开销更低。但是PMU方法依赖具体硬件实现，并且需确保进程具备CAP_PERFMON权限（Linux 5.8+）或CAP_SYS_ADMIN权限。而且PMU无法像SIGPROF那样可以通过handler获取更丰富的信息。

#### GPU 上的采样技术

CUDA支持对设备上的PC指针（program counter）进行采样，每个SM（Streaming Multiprocessor）以固定的时间间隔，随机选择一个wrap记录其调度状态与PC指针。同时CUDA也提供关联PC指针与SASS代码的方法，共用户获取函数ID。

自CUDA 12.6版本开始，CUPTI（CUDA Profiling Toolkit Interface）引入了全新的performance monitors (PM)接口[^cupti_pm]，基于该接口可以获取：
- GR Active: The percentage of cycles the compute engine is active；
- Tensor Active / FP16 Active: The ratio of cycles the SM tensor pipes or FP16x2 pipes were active issuing tensor instructions to the number of cycles in the sample period as a percentage.
- DRAM Read Bandwidth: The ratio of cycles the DRAM interface was active reading data to the elapsed cycles in the same period as a percentage.
- PCIe Read Throughput: The ratio of bytes received on the PCIe interface to the maximum number of bytes receivable in the sample period as a percentage. 
- NVLink bytes received: The ratio of bytes received on the NVLink interface to the maximum number of bytes receivable in the sample period as a percentage.

完整的列表请参考Nvidia官方文档。

### Performance Counter方法

从Instrumentation和Sampling方法的介绍中，我们可以看到一个趋势，基于采样的方法因为其侵入性低、适用范围广，得到了越来越多的发展与硬件支持。除了常规的IP指针采样以外，内存操作计数器，PCIE操作计数器等也为性能问题的诊断提供了大量有用信息。但性能计数器方法的实现也不仅仅限于硬件实现，软件层面也能通过性能计数器提供诊断信息。此处简单列举了常见的软硬件计数器：

#### 硬件计数器

在一个异构训练的硬件系统中，不同的硬件组件有各自不同的计数器，以下列举常见的硬件计数器：

- CPU[^1][^2]
  - 指令计数器：`instructions`指令计数器与`cycles`时钟计数器等；
  - 缓存：`cache-misses`L1、L2、L3 缓存未命中的次数与`cache-references`缓存访问次数等;
  - 分支：`branch-instructions`分支指令次数与`branch-misses`分支预测失败次数等；
- GPU[^3][^4][^5]
  - 计算：`sm_inst_executed`执行的指令数, `sm_inst_executed_atomics`执行的原子指令数；
  - 访存：`sm_inst_executed_generic_loads`访存load指令数和`sm_inst_executed_generic_stores`访存save指令数；
- RDMA[^6]
  - 基础统计：`port_rcv_packets`接收包数和`port_xmit_packets`（发送包数等；
  - 数据：`port_rcv_data`接收数据量和`port_xmit_data`发送数据量；
  - 拥塞控制：`np_cnp_sent`等；

#### 软件计数器
在整个软件栈上的各个层面上，也同样存在大大小小的计数器：

- OS层面：
  - /proc/<pid>/stat， CPU、上下文切换、系统调用等统计信息；
  - /proc/<pid>/statm，进程的内存使用情况；
- 框架层面：
  - NCCL Counter[^7]
  - PyTorch Flight Recorder[^8]

## 分布式训练Profiling的挑战与应对策略

现有的 Profiling 工具能够较好地解决单机性能分析问题，但在分布式训练场景下，它们面临全新的挑战：

1. **性能特征的剧烈变化**  
   - 当集群规模扩大时，训练任务的性能特性会显著转变。例如：
     - 3D 并行切分策略受集群规模及并行参数的影响，单机Profiling难以捕捉到所有参与角色的关键指标；
     - 大规模集群中，集合通信可能引发延迟激增和带宽利用率下降的问题，单机Profiling工具难以准确反映这些现象。

2. **数据统计与解析的复杂性**  
   - 单机Profiling通常依靠单一样本得出较为确定的结论，而分布式环境更多依赖海量统计数据，这大大增加了结果的准确性要求和数据解析的复杂度。

3. **跨节点协同调试需求**  
   - 分布式环境中，各节点之间必须实现精准的协调，对不同角色间的协同行为进行实时且深入的分析，这对Profiling工具提出了更高要求。
   - timeline 对于单机系统性能分析具有极大的帮助，但是对于分布式训练问题，timeline方法会面临两方面的问题：

     1. 数据量与分析难度：在1000节点集群中，Timeline数据量可能超过TB级别，导致存储与可视化不可行，并且1000条timeline也难以分析；
     2. 时间精度问题：分布式节点时间同步存在精度问题，且毫秒级时间同步误差会掩盖真实通信问题。

**应对这些挑战的技术路线：**

1. **以Sampling为主的低侵入Profiling**  
   - 通过灵活调整采样率，可以在保证系统整体性能的前提下，适应从单机调试到大规模分布式生产任务的Profiling需求。

2. **Instrumentation与Sampling的混合方案**  
   - 针对执行时间较长、低频任务，在采样的基础上混入Instrumentation手段，从而既保证关键数据的准确性，又提供足够的上下文信息以辅助精细调优。

3. **采样与性能建模的结合**  
   - 由于纯Sampling方法不足以展示完整的执行timeline，有必要引入性能建模：
     - 对训练任务的理论FLOPS（Floating-Point Operations Per Second）、内存吞吐和通信量进行建模；
     - 利用建模结果与采样数据相结合，量化各模型层的算力利用率和带宽利用率，从而全面评估计算、通信及访存之间的潜在掩盖现象。

## Probing的性能Profiling方案

### 方案介绍

1. 将模型的执行按照其Layer结构，拆分成不同的span；这个过程可以借助Torch与Python的反射特性自动完成；
2. 对于每个span，以采样的方式对其进行计时，同时记录span相关元数据，比如span内的计算内容、本次计算的输入等；
3. 对于每个span，在采样执行时间的同时，对底层硬件计数器，比如NCCL通信和访存等进行采样；
4. 结合性能建模结果，评估每个span内部的实际硬件吞吐是否合理，并计算span内的算力利用率、内存带宽利用率和互联带宽利用率；

probing给出的profiling方案本质上是通过纵向分层解耦分析代替横向时间轴关联分析：

|          | timeline方法               | probing                        |
| -------- | -------------------------- | ------------------------------ |
| 分析维度 | 横向时间轴关联分析         | 纵向分层解耦分析               |
| 关注焦点 | 跨节点事件的时间顺序关系   | 单节点各抽象层的资源利用效率   |
| 数据依赖 | 需要精确的全局时钟同步     | 仅需相对时间戳或逻辑因果关系   |
| 典型输出 | Gantt图、通信时序图        | 蜂窝热力图、分层利用率雷达图   |
| 适用场景 | 死锁诊断、精细化的通信优化 | 常态化性能监控、架构级瓶颈定位 |

### 如何解决问题

#### 单机环境下基础性能问题

- 定位低效算子：
  - 传统难点：可以观测到每个算子的时间，但是无法确定合理时间时多少，有多少优化空间；
  - 解决问题：
    - 通过Probing自定向下分析每个span的MFU；
    - 自顶层span递降定位MFU较低的细粒度span；
- 任务流水线掩盖不足，计算与数据拷贝未充分重叠；
  - 传统难点：Timeline工具可观测事件重叠，但无法量化掩盖效率。
  - 解决问题：
    - 定义流水线阶段理想执行时间（如T_pipeline = max(T_load, T_comp, T_save)
    - 计算实际效率（效率 = T_pipeline理想值 / T_pipeline实际值），识别拖尾阶段。
  
#### 小规模集群下分布式调优

- 通信-计算掩盖失效
  - 传统难点：通信耗时受网络拓扑、消息大小、协议类型等多因素影响，难以定位根因
  - 解决问题：
    - 通过Probing获取某个span的执行时间；
    - 通过性能建模计算理想计算时间与理想通信时间；
    - 计算通信时间暴露比 = (T_span - max(T_comp, T_c2c))/T_c2c;
  - 负载不均衡引发的集体操作延迟
    - 传统难点：无法确定各个节点进入集合通信语义的时间；
    - 解决问题：
      - 追踪每次集合通信开始到结束的时间；
      - 根据节点角色，对节点分组，分组内比较集合通信的时间开销；

#### 超大规模训练的独有难题

- 训练hang住
  - 传统难点：难以在几千个进程中定位到哪个进程导致hang；
  - 解决问题：
    - 通过probing查看每个进程的调用堆栈；
    - 通过probing查看每个进程内集合通信库的内部计数器；

- 慢节点问题
  - 传统难点：缺乏手段定位哪个进程速度慢；
  - 解决稳定：
    - 借助probing绘制performance heatmap，在heatmap上定位慢节点；
    - 通过通信库的性能计数器，定位经常被其他节点等待的节点；

## 参考文献

[^1]: https://github.com/torvalds/linux/blob/master/tools/perf/Documentation/perf-intel-pt.txt
[^2]: perf list 可查看完整列表
[^3]: [Nvidia显卡的硬件性能计数器](https://docs.nvidia.com/gameworks/index.html#developertools/desktop/linux_graphics_debugger/lgd_perf_counters.htm)
[^4]: [NSight Compute中的硬件Metric](https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html#metrics-reference)
[^5]: [PTX中的PMU寄存器](https://docs.nvidia.com/cuda/parallel-thread-execution/index.html#special-registers-pm0-pm7)
[^6]: [RDMA网卡的硬件计数器](https://enterprise-support.nvidia.com/s/article/understanding-mlx5-linux-counters-and-status-parameters);
[^7]: https://github.com/NVIDIA/nccl/blob/master/src/include/nvtx3/nvtxDetail/nvtxExtImplCounters_v1.h
[^8]: https://pytorch.org/tutorials/prototype/flight_recorder_tutorial.html#enabling-flight-recorder
[^9]:https://man7.org/linux/man-pages/man2/perf_event_open.2.html
[^setitimer]: https://linux.die.net/man/2/setitimer
[^cupti_pm]: https://docs.nvidia.com/cupti/main/main.html#cupti-pm-sampling-api