# Summary of Signal Handler Safety Improvements

## 问题概述 (Problem Summary)

本PR解决了在信号处理器中调用 `backtrace-rs` 的安全性问题。

This PR addresses the safety concerns of calling `backtrace-rs` from signal handlers.

## 改动内容 (Changes Made)

### 1. 详细文档 (Comprehensive Documentation)

#### `docs/signal-handler-safety.md` (365行中英文文档)
- **为何使用信号** (Why use signals):
  - 异步中断能力
  - 线程上下文访问
  - 低开销
  - 工业标准
  
- **风险说明** (Risks explained):
  - backtrace-rs 不是异步信号安全的
  - 可能导致死锁、内存损坏、崩溃
  - 具体场景分析和实例
  
- **四种修复方案** (Four fix solutions):
  1. 两阶段收集 (Two-phase collection)
  2. 异步信号安全的栈捕获 (Async-signal-safe stack capture) - **推荐**
  3. 使用专门的库 (Use dedicated libraries)
  4. 使用 pprof (Use pprof) - **生产环境推荐**

#### `probing/extensions/python/src/features/SIGNAL_SAFETY.md`
- 模块级别的安全说明
- 如何切换到更安全的实现
- 生产环境建议
- 已知问题和限制

### 2. 代码改进 (Code Improvements)

#### `probing/extensions/python/src/features/stack_tracer.rs`
新增 230 行代码，包括：

1. **详细的内联文档**:
   - `SignalTracer` 结构体的完整说明
   - `backtrace_signal_handler()` 的详细安全警告
   - 解释为什么接受这些风险（实用主义）

2. **实验性的安全实现** (x86_64 Linux/macOS):
   ```rust
   // 异步信号安全的原始帧捕获
   unsafe fn capture_raw_frames_signal_safe() -> usize
   
   // 在安全上下文中解析符号
   fn resolve_raw_frames(frame_count: usize) -> Vec<CallFrame>
   
   // 可选的安全信号处理器
   pub fn backtrace_signal_handler_safe()
   ```

3. **关键特性**:
   - 使用预分配的 `AtomicPtr` 数组（无malloc）
   - 仅在信号处理器中捕获帧指针
   - 延迟符号解析到安全上下文
   - 需要启用帧指针: `RUSTFLAGS="-C force-frame-pointers=yes"`

### 3. 测试 (Testing)

#### `tests/test_signal_handler_safety.py` (207行)
集成测试验证：
- 基本信号处理器功能
- 重复信号传递不会导致挂起
- 进程正常完成无死锁

### 4. 主文档更新 (Main Documentation Updates)

#### `README.md` 和 `README.cn.md`
- 添加信号处理器安全性警告
- 推荐生产环境使用 pprof
- 链接到详细文档

## 设计决策 (Design Decisions)

### 保持默认行为不变
- 当前实现继续使用 `backtrace::trace()` 在信号处理器中
- **原因**: 
  - 在大多数情况下工作良好
  - 简单直接
  - 与工业标准profiler（perf, pprof等）的做法一致

### 提供更安全的可选实现
- 实验性的 `backtrace_signal_handler_safe()`
- 用户可以选择启用
- **权衡**:
  - 更安全但需要帧指针
  - 平台特定（x86_64）
  - 需要额外配置

### 重点放在文档和教育
- 清楚说明风险
- 提供多种解决方案
- 允许知情决策

## 影响分析 (Impact Analysis)

### ✅ 积极影响
1. **透明度**: 用户现在了解潜在风险
2. **选择**: 提供多种实现选项
3. **教育**: 详细的技术文档
4. **生产建议**: 推荐使用 pprof

### ✅ 无负面影响
1. **向后兼容**: 默认行为未改变
2. **性能**: 无性能影响
3. **功能**: 所有现有功能保持不变

### ⚠️ 需要注意
1. **文档维护**: 需保持文档更新
2. **用户教育**: 用户需要理解权衡
3. **测试覆盖**: 安全实现需要更多测试

## 编译验证 (Compilation Verification)

```bash
✓ cargo check -p probing-python - PASSED
✓ cargo check -p probing-cli - PASSED  
✓ cargo check -p probing-core - PASSED
✓ Python import test - PASSED
```

## 文件变更统计 (File Changes Statistics)

```
README.cn.md                                            |   6 +
README.md                                               |  11 +
docs/signal-handler-safety.md                           | 365 ++++++++++++
probing/extensions/python/src/features/SIGNAL_SAFETY.md | 115 ++++
probing/extensions/python/src/features/stack_tracer.rs  | 230 ++++++++
tests/test_signal_handler_safety.py                     | 207 +++++++
6 files changed, 934 insertions(+)
```

## 后续工作 (Future Work)

### 短期 (Short-term)
- [ ] 收集用户反馈
- [ ] 在更多平台上测试安全实现
- [ ] 添加性能基准测试

### 中期 (Medium-term)
- [ ] 扩展安全实现到 ARM64
- [ ] 考虑集成 libunwind 作为替代
- [ ] 改进错误处理和恢复

### 长期 (Long-term)
- [ ] 评估是否默认切换到安全实现
- [ ] 与 backtrace-rs 上游合作改进信号安全性
- [ ] 探索使用 eBPF 作为替代采集方法

## 参考资料 (References)

1. [POSIX Async-Signal-Safety](https://man7.org/linux/man-pages/man7/signal-safety.7.html)
2. [backtrace-rs Documentation](https://docs.rs/backtrace/)
3. [pprof-rs Signal Safety](https://github.com/tikv/pprof-rs)
4. [Linux Signal Handling Best Practices](https://www.gnu.org/software/libc/manual/html_node/Defining-Handlers.html)

## 结论 (Conclusion)

本PR通过以下方式全面解决了问题陈述中提出的三个问题：

This PR comprehensively addresses the three questions raised in the problem statement:

1. ✅ **为何使用信号采集native堆栈**: 详细文档解释了原因（异步中断、线程上下文、低开销等）
2. ✅ **信号处理器中调用backtrace-rs的风险**: 完整的风险分析和实例说明
3. ✅ **修复方案**: 提供了四种不同的解决方案，包括一个实验性的安全实现

所有更改都经过仔细设计，确保向后兼容性，同时为希望最大化安全性的用户提供选项。
