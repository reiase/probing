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

# 使用 Probing 进行代码调试

本文介绍如何通过 Probing 进行 Python 应用代码调试，关于 Probing 的整体架构设计，请参考[架构概览](../advanced/architecture.md)。

系统调试一直是分布式系统开发和优化的难题，尤其是异构分布式训练系统，需要综合定位从硬件、系统到框架、模型等多个层面的错误和问题。对于分布式系统的调试需求主要集中在几方面：

1. 断点：断点是调试程序的最基本手段，通过断点可以用来观察程序状态、变量值，进而帮助BUG的分析与解决，断点又分为两类：
   1. 位置断点：用户指定在特定函数或者特定代码的特定位置中断程序执行，并进入调试器；
   2. 条件断点：用户指定某个变量或者内存地址，当发生变化时触发程序中断；
2. 插桩：通常是在目标代码位置插入日志，查看变量值或者系统状态；
3. 现场捕获：当出现异常时，第一时间捕获现场，或者触发断点，供进一步调试；

## Debug方法论

### CPU Debug方法的实现

本节讨论CPU侧Debugger的实现方法，用于参考和借鉴。

#### 如何控制目标进程

当调试一个进程时，第一步是如何获得目标进程的控制权限，控制目标进程暂停或者恢复执行。这一步在Linux系统中主要通过ptrace系统调用实现，以下是ptrace系统调用的函数原型：
```C
#include <sys/ptrace.h>

long ptrace(enum __ptrace_request op, pid_t pid,
            void *addr, void *data);
```

`ptrace` 提供了一种控制目标进程执行的方法，它可以让调试器与目标进程进行交互，从而实现调试功能。__ptrace_request常用的取值如下：

- `PTRACE_ATTACH`: 附加到目标进程，使其成为当前进程的tracee；
- `PTRACE_INTERRUPT`: 暂停目标tracee；
- `PTRACE_CONT`: 让目标进程继续执行；
- `PTRACE_DETACH`: 释放目标tracee；
- `PTRACE_GETREGS/PTRACE_SETREGS`: 读写目标进程寄存器；
- `PTRACE_PEEKDATA/PTRACE_POKEDATA`： 读写目标进程内存，一次一个WORD；
- `/proc/<pid>/mem`: 大块读写内存；

常见的一个debugger的工作流程如下：
1. attach到目标进程；
2. 通过读写目标进程TEXT段插入断点；
3. 恢复目标进程执行，并用`waitpid`等待目标进程断点暂停；
4. 等到目标进程暂停，通过读写内存查看信息；

#### CPU断点调试

##### 软件断点

X86处理器支持一个特殊的中断指令(INT 3, 0xCC)，当CPU执行到该指令时，会触发中断让调试器捕获。插入断点需要直接修改目标进程的代码段。

##### 硬件断点

X86处理器提供一系列调试寄存器(DR0-DR3)[^dr0_3]，可以在不修改内存的情况下，监视寄存器指向的地址。当目标地址被访问或者执行时，会触发CPU的中断，通知GDB。寄存器定义如下[^x86_regs]：

- DR0-DR3：断点的虚拟地址
- DR6：状态寄存器，存放DEBUG相关的状态指示位；
- DR7：控制寄存器，控制断点相关行为

### Python调试方法的实现

#### trace机制

Python 自身的调试器主要通过trace机制[^pytrace]实现:

```python
sys.settrace(tracefunc)
```

`tracefunc`会从Python解释器接收5种事件：

- call：执行函数调用；
- line：执行一行代码；
- return：函数返回；
- exception：发生一场；
- opcode：执行一条字节码；由于opcode的trace性能开销过大，需要设置`f_trace_opcodes`才能开启

借助`trace`机制，可以实现对Python变量的watch方法：

```python
def trace(self, frame: FrameType, event: AnyStr, arg: Any):
    if event != "line": return self.trace # 忽略line以外事件

    for k, v in self.watch.items(): # 遍历watch列表
        if k in frame.f_locals and id(frame.f_locals[k]) != v: # 检测变量id变化
            print(f"variable update {k} = {frame.f_locals[k]}")
            self.watch[k] = id(frame.f_locals[k])
    return self.trace # 继续跟踪, 每次需要返回一个新的trace函数
```

#### 执行事件监控机制

自3.12起，Python引入了Execution event monitoring机制[^monitoring]，用于向各种工具提供Python解释器的内部执行事件。这个机制主要面向Debugger、Profiler和Optimizer开发者提供。我们先来看一个简单的例子，通过`sys.monitoring`来捕获Python执行过程中的异常：

```python
import sys

def hook(*args, **kwargs): # 定义事件hook
    print("=== hook", args, kwargs)

# 声明调试工具
sys.monitoring.use_tool_id(sys.monitoring.DEBUGGER_ID, "debugging")

# 启用异常的RAISE事件
sys.monitoring.set_events(sys.monitoring.DEBUGGER_ID, sys.monitoring.events.RAISE)

# 为异常事件注册钩子
sys.monitoring.register_callback(
    sys.monitoring.DEBUGGER_ID,
    sys.monitoring.events.RAISE,
    hook,
)

# 测试代码
def foo(a):
    b = 2
    bar(a, b)

def bar(a, b):
    c = a+b
    raise Exception('error')

foo(1)
```
在执行上述代码，会在`bar`函数触发异常时，打印如下消息：
```
=== hook (<code object bar at 0x104bbcb70, file "b.py", line 24>, 32, Exception('error')) {}
```

##### 注册和使用tools

monitoring中一个核心概念是 **tool**，用于区分不同的工具，避免彼此冲突。**tool**相关的API如下：
```
sys.monitoring.use_tool_id(tool_id: int, name: str, /) → None¶
声明使用**tool**，必须在使用前进行声明；

sys.monitoring.free_tool_id(tool_id: int, /) → None¶
释放**tool**

sys.monitoring.get_tool(tool_id: int, /) → str | None¶
获取**tool**
```

**tool**可以用任意整数ID来定，方便起见，系统预定义了几个tool的ID：
```
sys.monitoring.DEBUGGER_ID = 0
sys.monitoring.COVERAGE_ID = 1
sys.monitoring.PROFILER_ID = 2
sys.monitoring.OPTIMIZER_ID = 5
```

##### 事件

定义了哪些执行阶段的事件会被发送给**tool**，目前支持的事件有
- **BRANCH**(sys.monitoring.events.BRANCH)
- **CALL**: A call in Python code (event occurs before the call).
- **C_RAISE**: An exception raised from any callable, except for Python functions (event occurs after the exit).
- **C_RETURN**: Return from any callable, except for Python functions (event occurs after the return).
- **EXCEPTION_HANDLED**: An exception is handled.
- **INSTRUCTION**: A VM instruction is about to be executed.
- **JUMP**: An unconditional jump in the control flow graph is made.
- **LINE**: An instruction is about to be executed that has a different line number from the preceding instruction.
- **PY_RESUME**: Resumption of a Python function (for generator and coroutine functions), except for throw() calls. 
- **PY_RETURN**: Return from a Python function (occurs immediately before the return, the callee's frame will be on the stack).
- **PY_START**: Start of a Python function (occurs immediately after the call, the callee's frame will be on the stack)
- **PY_THROW**: A Python function is resumed by a throw() call.
- **PY_UNWIND**: Exit from a Python function during exception unwinding.
- **PY_YIELD**: Yield from a Python function (occurs immediately before the yield, the callee's frame will be on the stack).
- **RAISE**: An exception is raised, except those that cause a STOP_ITERATION event.
- **RERAISE**: An exception is re-raised, for example at the end of a finally block.
- **STOP_ITERATION**: An artificial StopIteration is raised; see the STOP_ITERATION event.

事件支持bool类型的操作，比如 `PY_RETURN | PY_START` 表示同时处理两种事件。

##### 注册回调函数

```python
sys.monitoring.register_callback(tool_id: int, event: int, func: Callable | None, /) → Callable | None
```
用于注册回调函数，返回值为老的回调函数。如果需要取消回调函数的注册，可以将参数`func`设置为`None`。

##### 对特定函数使能Event

除了全局使能event以外，还可以针对某些函数来使能event
```python
sys.monitoring.set_local_events(tool_id: int, code: CodeType, event_set: int, /) → None¶
```
这里`code`参数可以是某个函数。局部使能event可以有效缩小事件的影响范围，降低性能影响。

### Exception捕获

```python
sys.excepthook(type, value, traceback)¶
```

通过注册`excepthook`，可以捕获Python进程的异常退出，捕获住未被捕获的异常，比如：
```python
import sys

def handle(t, v, tb):
    print("=== exception handler, ", t, v, tb)

def foo(a):
    b = 2
    bar(a, b)

def bar(a, b):
    raise Exception('error')

sys.excepthook = handle

foo(1)
```

`excepthook`可以捕获堆栈，但是无法捕获现场与局部变量。

## 分布式训练调试的挑战

### 典型场景

分布式异构训练通常面临如下场景：

- 数据并行：多个节点同时处理不同的数据子集，并在参数服务器或环形通信中同步梯度。
- 模型并行：将模型分为不同部分，分别运行在不同的节点上。
- 异构训练：训练使用GPU这种异构计算设备，在每个节点上都与CPU异步执行。
- 故障恢复：在分布式训练中，节点故障可能频繁发生，需要有效的故障定位与恢复机制。


1. 分布式断点，观察每个进程的执行情况；
2. 分布式变量观测，观测每个进程的关键变量；
3. 分布式hook：关注关键变量或者tensor的取值与变化情况；
4. 分布式backtrace：观测每个进程的执行堆栈；

### 主要挑战

1. 分布式断点调试：
   - 多节点间存在角色差异(不如不同的TP/PP/DP角色)
   - 集合通信场景下的断点会影响整体执行流
   - 传统断点调试工具难以适应分布式环境
   - 需要设计轻量级且语义感知的观测机制

2. 分布式变量观测：
   - 变量获取与序列化开销大
   - 网络传输带宽受限
   - 数据聚合与展示复杂
   - 实时性与系统性能之间的权衡

3. 分布式hook与traceback
   - 捕获关键动作或变量更新，特别是通信库中的同步原语与异步逻辑；
   - 获取每个进程的执行堆栈，并汇总分析；

4. 规模扩展性
   - 日志数据量随节点数激增
   - 调试信息的汇总与分析困难
   - 存储与查询性能瓶颈
   - 可视化呈现挑战大

## Probing的分布式Debug方案

Probing目前提供的一些机制能够较好的支撑分布式Debug能力的开发：

1. 分布式探针
   - Probing的探针可以起到C/C++的ptrace与Python的Trace的作用；
   - Probing的探针支持远程控制，可以用于控制集群所有探针；
   - 可以借助sys.monitoring，实现轻量级的trace，尽可能降低对性能的影响；

2. 数据处理
   - Probing自带Query引擎，善于处理大量本地数据；
   - Query引擎的分布式能力，能够帮助Probing自动管理集群层面的分布式数据处理；
   - Query引擎内部实现了高效的数据压缩；
  
3. 自动化
   - Query 引擎提供了编程能力；
   - 标准化的SQL查询语句，可以借助大模型自动生成SQL；

### 高性能tracer

针对Python 3.12 以上版本，可以借助`sys.monitoring` 实现高效的tracer。在tracer实现的过程中，应该遵循最小scope原则，即控制tracer所影响的Python代码，让其越小约好。

1. 控制生效范围：`sys.monitoring`支持基于字节码的trace，即通过`sys.monitoring.set_local_events`只对某个函数的字节码实现trace。
2. 控制时间类型：越细粒度的事件，trace开销越大。因此在选择事件的时候，应该遵循如下原则：
   1. 优先使用触发性事件，比如`RAISE`这种异常抛出事件；
   2. 其次考虑使用`PY_START`或者`RETUEN`这种函数级别事件；
   3. 最后考虑`LINE`这种细粒度事件；

针对Python 3.12 以前的版本，可以使用`trace`函数，但需要手动控制其影响范围：
1. 使用`with`语句控制trace范围内，保证tracer只对某个函数调用生效：

```python
def probe(func=None):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        tracer = ProbingTracer(depth, watch)
        with tracer:
            return func(*args, **kwargs)

    return wrapper
```

2. 控制trace深度，及时关闭trace

```python
# 向调用栈注入trace guard
frame.f_locals["__trace_guard__"] = TracerGuard()

class TracerGuard:
    def __init__(self, callback=None):
        self.trace = sys.gettrace() # 保存trace函数
        sys.settrace(None)          # 禁用trace

    def __del__(self):
        sys.settrace(self.trace)    # 恢复trace
```

### 变量观测与hook

针对变量变更的追踪，可以通过以下方式实现：
1. 创建一个Tensor类型的子类；
2. 通过torch的dispatch机制，让对tensor的修改计算，dispatch给`__torch_function__`
3. 通过tracer，将调用堆栈上感兴趣的tensor替换成`HookedTensor`;

```python
class HookedTensor(torch.Tensor, FakeProbingTensor):
    def __format__(self, format_spec):
        return f"{self.item().__format__(format_spec)}"

    @classmethod
    def __torch_function__(cls, func, types, args=(), kwargs=None):
        if kwargs is None:
            kwargs = {}
        if (
            func is not torch.Tensor.__repr__
            # and func is not torch.Tensor.__format__
            and func is not torch.Tensor.__str__
            and func.__name__.endswith("_")
            and not func.__name__.startswith("__")
        ):
            old_val = f"{args}"
            ret = super().__torch_function__(func, types, args, kwargs)
            ret_val = f"{args}"
            print(
                f"probing: tensor update with {func.__name__}: {old_val} => {ret_val}"
            )
            return ret
        return super().__torch_function__(func, types, args, kwargs)
```

## 实际调试案例

### 案例1：内存泄漏调试

使用 Probing 的内存监控功能来定位 Python 应用中的内存泄漏：

```python
# 启用内存追踪
import probing

# 在训练循环中监控内存使用
with probing.trace("memory_usage"):
    for epoch in range(num_epochs):
        train_model(epoch)
        
# 通过SQL查询分析内存使用模式
```

### 案例2：分布式训练同步问题

监控分布式训练中的通信和同步问题：

```python
# 监控集合通信操作
with probing.trace("collective_ops"):
    torch.distributed.all_reduce(tensor)
    
# 分析不同节点的执行时间差异
```

## 最佳实践

1. **最小化性能影响**：只在必要的代码段启用追踪
2. **合理选择事件类型**：根据调试需求选择合适的监控事件
3. **利用SQL分析**：使用Probing的SQL接口进行高效的数据分析
4. **分布式协调**：在分布式环境中注意同步和协调问题

## 参考链接

- [内存分析指南](memory-analysis.md)
- [分布式训练分析](distributed.md)
- [SQL分析接口](sql-analytics.md)

[^ptrace]: https://www.man7.org/linux/man-pages/man2/ptrace.2.html
[^dr0_3]: https://sandpile.org/x86/drx.htm
[^x86_regs]: https://wiki.osdev.org/CPU_Registers_x86#DR0_-_DR3
[^pytrace]: https://docs.python.org/3/library/sys.html
[^monitoring]: https://docs.python.org/3/library/sys.monitoring.html#module-sys.monitoring
