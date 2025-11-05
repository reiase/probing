# Signal Handler Safety: Native Stack Collection

## 为何使用信号采集native堆栈 (Why Use Signals for Native Stack Collection)

### 问题背景 (Background)

在性能分析和调试工具中,我们经常需要获取目标线程的调用栈。对于Python堆栈,我们可以通过访问Python解释器的内部数据结构来获取。但对于native (C/C++) 堆栈,我们需要一种机制来中断目标线程并捕获其当前执行状态。

In performance profiling and debugging tools, we often need to capture the call stack of a target thread. For Python stacks, we can access the Python interpreter's internal data structures. However, for native (C/C++) stacks, we need a mechanism to interrupt the target thread and capture its current execution state.

### 为什么使用信号 (Why Use Signals)

信号(Signal)是Unix/Linux系统中用于进程间通信和中断处理的机制。我们使用`SIGUSR2`信号来采集native堆栈,原因如下:

Signals are a Unix/Linux mechanism for inter-process communication and interrupt handling. We use the `SIGUSR2` signal to collect native stacks for the following reasons:

1. **异步中断能力 (Asynchronous Interruption)**
   - 信号可以异步中断目标线程的执行
   - 不需要目标线程的配合或轮询
   - Can asynchronously interrupt the target thread's execution
   - Does not require cooperation or polling from the target thread

2. **线程上下文 (Thread Context)**
   - 信号处理器在目标线程的上下文中运行
   - 可以直接访问该线程的栈帧和寄存器
   - Signal handler runs in the context of the target thread
   - Can directly access that thread's stack frames and registers

3. **低开销 (Low Overhead)**
   - 相比其他机制(如ptrace),信号的开销较小
   - 适合高频采样的性能分析
   - Lower overhead compared to other mechanisms (like ptrace)
   - Suitable for high-frequency sampling in performance profiling

4. **标准接口 (Standard Interface)**
   - POSIX标准定义的机制
   - 跨平台兼容性好
   - POSIX standard defined mechanism
   - Good cross-platform compatibility

## 在信号处理器中调用backtrace-rs的风险 (Risks of Calling backtrace-rs in Signal Handlers)

### 核心问题: 非异步信号安全 (Core Issue: Not Async-Signal-Safe)

**重要**: `backtrace-rs` 库中的 `backtrace::trace()` 和 `backtrace::resolve_frame()` 函数**不是**异步信号安全的(async-signal-safe)。

**IMPORTANT**: The `backtrace::trace()` and `backtrace::resolve_frame()` functions in the `backtrace-rs` library are **NOT** async-signal-safe.

### 什么是异步信号安全 (What is Async-Signal-Safety)

POSIX标准规定,只有特定的函数可以在信号处理器中安全调用。一个函数是异步信号安全的,需要满足:

POSIX standards specify that only certain functions can be safely called from signal handlers. A function is async-signal-safe if:

1. **可重入性 (Reentrancy)**: 函数不使用静态或全局数据,或者使用原子操作保护
2. **无锁等待 (No Lock Waiting)**: 不会等待锁或互斥量
3. **无内存分配 (No Memory Allocation)**: 不调用 `malloc()`, `free()` 等
4. **无I/O操作 (No I/O Operations)**: 不执行可能阻塞的I/O
5. **无信号屏蔽 (No Signal Masking)**: 不修改信号掩码

### backtrace-rs 的非安全行为 (Unsafe Behaviors in backtrace-rs)

`backtrace::trace()` 和 `backtrace::resolve_frame()` 会执行以下非异步信号安全的操作:

`backtrace::trace()` and `backtrace::resolve_frame()` perform the following non-async-signal-safe operations:

1. **内存分配 (Memory Allocation)**
   ```rust
   // backtrace-rs internally calls malloc/free
   backtrace::trace(|frame| {
       // Allocates memory for frame data
       // symbol.name() may allocate strings
   });
   ```
   - 如果主线程正在执行 `malloc()`,信号处理器中再次调用会导致死锁
   - If the main thread is in `malloc()`, calling it again in the signal handler causes deadlock

2. **锁竞争 (Lock Contention)**
   ```rust
   // Symbol resolution may acquire locks
   backtrace::resolve_frame(frame, |symbol| {
       // Internal locks for symbol cache, etc.
   });
   ```
   - 可能与主线程的锁产生竞争,导致死锁
   - May compete with locks in the main thread, causing deadlock

3. **I/O操作 (I/O Operations)**
   - 符号解析可能需要读取 `/proc/self/maps`, `.debug` 文件等
   - Symbol resolution may need to read `/proc/self/maps`, `.debug` files, etc.
   - 文件I/O不是异步信号安全的
   - File I/O is not async-signal-safe

4. **复杂的C++代码 (Complex C++ Code)**
   ```rust
   cpp_demangle::Symbol::new(raw_name)  // Not async-signal-safe
   ```
   - C++ name demangling涉及复杂的字符串操作和内存管理
   - C++ name demangling involves complex string operations and memory management

### 实际风险和后果 (Real Risks and Consequences)

1. **死锁 (Deadlock)**
   ```
   Thread A: malloc() -> [holds lock] -> interrupted by SIGUSR2
             -> backtrace::trace() -> malloc() -> [waits for lock] -> DEADLOCK
   ```

2. **内存损坏 (Memory Corruption)**
   - 重入的内存分配器可能破坏堆结构
   - Reentrant memory allocator may corrupt heap structure

3. **数据竞争 (Data Races)**
   - 信号处理器和主线程同时访问共享数据
   - Signal handler and main thread accessing shared data simultaneously

4. **崩溃 (Crashes)**
   - 未定义行为可能导致段错误或其他崩溃
   - Undefined behavior may cause segfaults or other crashes

### 实例场景 (Example Scenario)

```
Timeline:
1. Thread is executing: malloc(size)        // Heap lock acquired
2. Signal arrives: SIGUSR2
3. Signal handler starts: backtrace_signal_handler()
4. Calls: backtrace::trace()
5. Internally calls: malloc()               // Attempts to acquire heap lock
6. DEADLOCK! (waiting for lock held by same thread)
```

## 修复方案 (Fix Solutions)

### 方案 1: 两阶段收集 (Two-Phase Collection) [当前实现 Current Implementation]

将堆栈收集分为两个阶段:

Split stack collection into two phases:

**Phase 1: 信号处理器(Signal Handler) - 仅设置标志**
```rust
static COLLECTION_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn backtrace_signal_handler() {
    // Only async-signal-safe operation
    COLLECTION_REQUESTED.store(true, Ordering::Release);
}
```

**Phase 2: 安全上下文(Safe Context) - 实际收集**
```rust
fn collect_after_signal() {
    if COLLECTION_REQUESTED.swap(false, Ordering::Acquire) {
        // Safe to call backtrace-rs here
        let stacks = SignalTracer::get_native_stacks();
        send_to_collector(stacks);
    }
}
```

**优点 (Advantages)**:
- 完全避免在信号处理器中调用非安全函数
- Completely avoids calling unsafe functions in signal handler

**缺点 (Disadvantages)**:
- 需要目标线程配合检查标志
- Requires target thread cooperation to check the flag
- 可能无法在线程被阻塞时工作
- May not work when thread is blocked

### 方案 2: 异步信号安全的栈捕获 (Async-Signal-Safe Stack Capture) [推荐 RECOMMENDED]

在信号处理器中只捕获原始帧指针,延迟符号解析:

Capture only raw frame pointers in signal handler, defer symbol resolution:

```rust
use std::sync::atomic::{AtomicPtr, Ordering};

const MAX_FRAMES: usize = 256;
static RAW_FRAMES: [AtomicPtr<std::ffi::c_void>; MAX_FRAMES] = 
    [const { AtomicPtr::new(std::ptr::null_mut()) }; MAX_FRAMES];
static FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn backtrace_signal_handler() {
    // Capture raw frame pointers using async-signal-safe method
    let mut count = 0;
    
    // Use frame pointer walking (async-signal-safe)
    unsafe {
        let mut rbp: *mut *mut std::ffi::c_void;
        std::arch::asm!("mov {}, rbp", out(reg) rbp);
        
        while !rbp.is_null() && count < MAX_FRAMES {
            let return_addr = rbp.offset(1).read();
            RAW_FRAMES[count].store(return_addr, Ordering::Relaxed);
            count += 1;
            rbp = rbp.read();
        }
    }
    
    FRAME_COUNT.store(count, Ordering::Release);
}

// Later, in safe context
fn resolve_captured_frames() -> Vec<CallFrame> {
    let count = FRAME_COUNT.swap(0, Ordering::Acquire);
    let mut frames = Vec::new();
    
    for i in 0..count {
        let ip = RAW_FRAMES[i].load(Ordering::Relaxed);
        // Now safe to use backtrace-rs for symbol resolution
        // ... resolve symbols ...
    }
    frames
}
```

**优点 (Advantages)**:
- 信号处理器完全异步信号安全
- Signal handler is completely async-signal-safe
- 不依赖backtrace-rs的内部实现
- Does not depend on backtrace-rs internals
- 可靠性高,适合生产环境
- High reliability, suitable for production

**缺点 (Disadvantages)**:
- 需要手动实现栈展开
- Requires manual stack unwinding
- 平台相关性(x86_64, ARM等不同)
- Platform-specific (differs for x86_64, ARM, etc.)

### 方案 3: 使用专门的异步信号安全库 (Use Dedicated Async-Signal-Safe Library)

使用 `backtrace-rs` 的 `trace_unsynchronized()` 或其他专门的库:

Use `backtrace-rs`'s `trace_unsynchronized()` or other dedicated libraries:

```rust
pub fn backtrace_signal_handler() {
    // backtrace_rs 0.3.69+ provides trace_unsynchronized
    // which is designed for signal handlers
    backtrace::trace_unsynchronized(|frame| {
        // Still not fully async-signal-safe, but better
        let ip = frame.ip();
        // Store only IPs, resolve later
        true
    });
}
```

**注意 (Note)**: 即使使用 `trace_unsynchronized()`, 符号解析仍然不是异步信号安全的。

Even with `trace_unsynchronized()`, symbol resolution is still not async-signal-safe.

### 方案 4: 使用 pprof 的实现 (Use pprof's Implementation)

`pprof` crate 已经正确处理了信号安全问题:

The `pprof` crate already handles signal safety correctly:

```rust
use pprof::ProfilerGuard;

// pprof handles signal safety internally
let guard = ProfilerGuard::new(100)?;

// ... run code ...

// Generate report
let report = guard.report().build()?;
```

**优点 (Advantages)**:
- 已经过生产验证
- Production-tested
- 处理了所有边界情况
- Handles all edge cases
- 提供完整的profiling功能
- Provides complete profiling features

**缺点 (Disadvantages)**:
- 可能不适合需要精确控制的场景
- May not suit scenarios requiring precise control
- 额外的依赖
- Additional dependency

## 推荐实现 (Recommended Implementation)

基于以上分析,推荐采用 **方案 2 + 方案 4 的混合方案**:

Based on the analysis above, we recommend a **hybrid approach combining Solution 2 and Solution 4**:

1. **对于连续性能分析 (For Continuous Profiling)**:
   - 使用 `pprof` crate
   - Use the `pprof` crate

2. **对于按需堆栈采集 (For On-Demand Stack Collection)**:
   - 使用方案2的两阶段方法,或者
   - Use Solution 2's async-signal-safe frame capture, or
   - 使用简化的帧指针捕获
   - Use simplified frame pointer capture

## 当前代码的状态 (Current Code Status)

**警告 (WARNING)**: 当前的实现在信号处理器中直接调用 `backtrace::trace()`,这是**不安全的**。

**WARNING**: The current implementation directly calls `backtrace::trace()` in the signal handler, which is **UNSAFE**.

```rust
// UNSAFE: Current implementation
pub fn backtrace_signal_handler() {
    let native_stacks = SignalTracer::get_native_stacks().unwrap_or_default();
    // ^^^ Calls backtrace::trace() - NOT async-signal-safe!
    // ...
}
```

**已知问题 (Known Issues)**:
- 可能导致死锁 (May cause deadlocks)
- 可能导致崩溃 (May cause crashes)
- 在高并发场景下不稳定 (Unstable under high concurrency)

## 迁移计划 (Migration Plan)

为了安全地迁移到新的实现:

To safely migrate to the new implementation:

1. **短期 (Short-term)**:
   - 添加文档警告
   - Add documentation warnings
   - 添加运行时检测机制
   - Add runtime detection mechanisms

2. **中期 (Medium-term)**:
   - 实现方案2的异步信号安全捕获
   - Implement Solution 2's async-signal-safe capture
   - 保持向后兼容
   - Maintain backward compatibility

3. **长期 (Long-term)**:
   - 完全迁移到安全实现
   - Fully migrate to safe implementation
   - 移除不安全的代码路径
   - Remove unsafe code paths

## 参考资料 (References)

1. [POSIX Async-Signal-Safety](https://man7.org/linux/man-pages/man7/signal-safety.7.html)
2. [backtrace-rs Documentation](https://docs.rs/backtrace/)
3. [pprof-rs Signal Safety](https://github.com/tikv/pprof-rs)
4. [Linux Signal Handling Best Practices](https://www.gnu.org/software/libc/manual/html_node/Defining-Handlers.html)

## 总结 (Summary)

- **为什么用信号**: 异步中断能力、线程上下文访问、低开销
- **Why use signals**: Asynchronous interruption, thread context access, low overhead

- **风险**: backtrace-rs不是异步信号安全的,可能导致死锁、崩溃
- **Risks**: backtrace-rs is not async-signal-safe, may cause deadlocks and crashes

- **修复**: 使用异步信号安全的帧捕获 + 延迟符号解析
- **Fix**: Use async-signal-safe frame capture + deferred symbol resolution
