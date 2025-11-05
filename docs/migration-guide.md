# 信号处理器升级迁移指南 (Signal Handler Migration Guide)

本文档提供详细的分步骤升级指南，帮助用户从当前的非异步信号安全实现迁移到更安全的实现。

This document provides a detailed step-by-step migration guide to help users migrate from the current non-async-signal-safe implementation to safer alternatives.

---

## 目录 (Table of Contents)

1. [前置评估 (Pre-Migration Assessment)](#前置评估)
2. [迁移路线选择 (Migration Path Selection)](#迁移路线选择)
3. [路线A: 迁移到pprof (推荐生产环境)](#路线a-迁移到pprof)
4. [路线B: 启用实验性安全处理器](#路线b-启用实验性安全处理器)
5. [路线C: 保持现状并监控](#路线c-保持现状并监控)
6. [测试和验证 (Testing and Validation)](#测试和验证)
7. [回滚计划 (Rollback Plan)](#回滚计划)
8. [性能对比 (Performance Comparison)](#性能对比)

---

## 前置评估 (Pre-Migration Assessment)

### 第一步: 评估当前使用场景

在开始迁移前，请回答以下问题：

Before starting migration, answer these questions:

1. **使用场景** (Usage Scenario):
   - [ ] 开发/调试环境 (Development/Debug)
   - [ ] 临时性能分析 (Ad-hoc profiling)
   - [ ] 持续性能监控 (Continuous profiling)
   - [ ] 生产环境诊断 (Production diagnostics)

2. **采样频率** (Sampling Frequency):
   - [ ] 低频 (< 1次/分钟) - Low risk
   - [ ] 中频 (1-10次/分钟) - Medium risk
   - [ ] 高频 (> 10次/分钟) - Higher risk

3. **目标平台** (Target Platform):
   - [ ] x86_64 Linux
   - [ ] x86_64 macOS
   - [ ] ARM64 Linux
   - [ ] 其他 (Other)

4. **是否遇到过以下问题** (Have you experienced):
   - [ ] 死锁或挂起 (Deadlocks or hangs)
   - [ ] 崩溃 (Crashes)
   - [ ] 性能下降 (Performance degradation)
   - [ ] 无 (None)

### 第二步: 风险评分

根据以上答案计算风险分数:

Calculate risk score based on above answers:

- 生产环境 + 高频采样 = **高风险** → 推荐**路线A** (pprof)
- 开发环境 + 中低频采样 = **中风险** → 可选**路线B**或**路线C**
- 开发环境 + 低频采样 + 无问题 = **低风险** → 推荐**路线C** (保持现状)

---

## 迁移路线选择 (Migration Path Selection)

### 路线对比表

| 特性 | 路线A: pprof | 路线B: 安全处理器 | 路线C: 保持现状 |
|------|-------------|-----------------|----------------|
| **安全性** | 最高 | 高 | 中 |
| **实施难度** | 低 | 中 | 无 |
| **性能开销** | 低 | 低 | 最低 |
| **功能完整性** | 完整 | 完整 | 完整 |
| **平台支持** | 全平台 | x86_64 only | 全平台 |
| **生产就绪** | 是 | 否(实验性) | 是 |
| **推荐场景** | 持续profiling | 安全要求高的开发 | 开发/调试 |

---

## 路线A: 迁移到pprof (推荐生产环境)

### 适用场景

- ✅ 生产环境持续性能监控
- ✅ 需要火焰图和详细分析
- ✅ 高采样频率场景
- ✅ 需要最高稳定性

### 迁移步骤

#### 1. 确认依赖 (已包含在 Cargo.toml)

```toml
[dependencies]
pprof = { version = "0.14.0", features = ["cpp", "flamegraph", "frame-pointer"] }
```

验证依赖:
```bash
cargo tree -p probing-python | grep pprof
```

#### 2. 创建 pprof 配置文件

创建 `config/pprof_config.toml`:

```toml
[pprof]
# 采样频率 (Hz) - 推荐 100-1000
frequency = 100

# 是否包含 C++ 符号
include_cpp = true

# 是否生成火焰图
enable_flamegraph = true

# 采样持续时间 (秒) - 0表示持续运行
duration = 0
```

#### 3. 修改代码启用 pprof

**方案3.1: 使用环境变量控制**

在 `probing/extensions/python/src/setup.rs` 中添加:

```rust
#[ctor]
fn setup() {
    // 检查是否应该使用 pprof
    if std::env::var("PROBING_USE_PPROF").is_ok() {
        log::info!("Using pprof for stack collection");
        // pprof 会自动注册信号处理器
        crate::features::pprof::pprof_handler();
    } else {
        log::info!("Using SignalTracer for stack collection");
        register_signal_handler(
            nix::libc::SIGUSR2,
            crate::features::stack_tracer::backtrace_signal_handler,
        );
    }
}
```

**方案3.2: 通过配置文件控制**

创建 `~/.probing/config.toml`:

```toml
[profiling]
backend = "pprof"  # 或 "signal_tracer"
sample_rate = 100
```

#### 4. 测试迁移

**步骤 4.1: 启动测试程序**

```bash
# 使用 pprof
PROBING=1 PROBING_USE_PPROF=1 python examples/test_probing.py &
TEST_PID=$!
```

**步骤 4.2: 验证 pprof 工作**

```bash
# 等待几秒让程序运行
sleep 5

# 生成火焰图
probing -t $TEST_PID query "SELECT * FROM pprof_flamegraph" > /tmp/flamegraph.svg

# 检查火焰图是否生成
ls -lh /tmp/flamegraph.svg
```

**步骤 4.3: 性能测试**

```bash
# 测试采样开销
time PROBING_USE_PPROF=1 python -c "
import time
for i in range(1000000):
    _ = i * i
"
```

#### 5. 生产部署

**步骤 5.1: 金丝雀部署**

先在 1-5% 的服务器上启用:

```bash
# 在部分服务器上设置环境变量
export PROBING_USE_PPROF=1

# 监控 1-2 天，观察:
# - CPU 使用率
# - 内存使用
# - 是否有崩溃或挂起
```

**步骤 5.2: 逐步扩展**

如果金丝雀阶段正常:
- 第一周: 25% 服务器
- 第二周: 50% 服务器  
- 第三周: 75% 服务器
- 第四周: 100% 服务器

**步骤 5.3: 监控指标**

设置告警监控:

```bash
# Prometheus 指标示例
- alert: PprofHighOverhead
  expr: probing_pprof_overhead_percent > 5
  for: 10m
  
- alert: PprofCollectionFailure
  expr: rate(probing_pprof_errors_total[5m]) > 0.1
```

---

## 路线B: 启用实验性安全处理器

### 适用场景

- ✅ 开发环境，需要最高安全性
- ✅ x86_64 Linux/macOS 平台
- ✅ 可以接受实验性功能
- ❌ 不推荐生产环境

### 前置要求

1. **必须**使用帧指针编译
2. 仅支持 x86_64 架构
3. 需要重新编译整个项目

### 迁移步骤

#### 1. 配置编译环境

创建 `.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "force-frame-pointers=yes"]

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "force-frame-pointers=yes"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "force-frame-pointers=yes"]
```

#### 2. 修改信号处理器注册

编辑 `probing/extensions/python/src/setup.rs`:

```rust
#[ctor]
fn setup() {
    // 使用更安全的信号处理器
    register_signal_handler(
        nix::libc::SIGUSR2,
        crate::features::stack_tracer::backtrace_signal_handler_safer,
    );
}
```

#### 3. 重新编译

```bash
# 清理之前的编译
cargo clean

# 使用帧指针重新编译
RUSTFLAGS="-C force-frame-pointers=yes" cargo build --release

# 验证帧指针已启用
nm target/release/libprobing_python.so | grep -i frame
```

#### 4. 构建 Python wheel

```bash
# 使用帧指针构建
RUSTFLAGS="-C force-frame-pointers=yes" python make_wheel.py

# 安装新的 wheel
pip install --force-reinstall dist/probing-*.whl
```

#### 5. 验证功能

**测试脚本** (`test_safer_handler.py`):

```python
import os
import sys
import time
import subprocess

def test_safer_handler():
    """测试更安全的信号处理器"""
    script = """
import time
def recursive_func(n):
    if n == 0:
        time.sleep(0.5)
        return
    return recursive_func(n - 1)

print("READY", flush=True)
recursive_func(10)
print("DONE", flush=True)
"""
    
    # 启动测试进程
    proc = subprocess.Popen(
        [sys.executable, "-c", script],
        env={**os.environ, "PROBING": "1"},
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # 等待就绪
    line = proc.stdout.readline()
    assert "READY" in line
    
    # 多次采样测试
    for i in range(10):
        result = subprocess.run(
            ["probing", "-t", str(proc.pid), "backtrace"],
            capture_output=True,
            timeout=5
        )
        print(f"Sample {i+1}: {'✓' if result.returncode == 0 else '✗'}")
        time.sleep(0.1)
    
    proc.wait()
    print("Test completed successfully!")

if __name__ == "__main__":
    test_safer_handler()
```

运行测试:

```bash
python test_safer_handler.py
```

#### 6. 性能对比测试

```bash
# 对比脚本
cat > benchmark_handlers.sh << 'EOF'
#!/bin/bash

echo "Testing default handler..."
time python -c "import time; [time.sleep(0.01) for _ in range(100)]"

echo "Testing safer handler..."
RUSTFLAGS="-C force-frame-pointers=yes"
time python -c "import time; [time.sleep(0.01) for _ in range(100)]"
EOF

chmod +x benchmark_handlers.sh
./benchmark_handlers.sh
```

---

## 路线C: 保持现状并监控

### 适用场景

- ✅ 开发/调试环境
- ✅ 低频采样 (< 1次/分钟)
- ✅ 未遇到稳定性问题
- ✅ 需要所有平台支持

### 监控措施

即使保持现状，也应该添加监控:

#### 1. 添加超时保护

在调用栈采集时添加超时:

```rust
// 在 stack_tracer.rs 的 trace() 方法中
let native_frames = rx.recv_timeout(Duration::from_secs(2))?;

// 建议改为更保守的值:
let native_frames = rx.recv_timeout(Duration::from_secs(5))?;
```

#### 2. 添加错误监控

创建 `monitor.py`:

```python
#!/usr/bin/env python3
"""监控 probing 采样失败率"""
import subprocess
import time
import sys

def monitor_sampling(pid, duration=60, interval=5):
    """监控指定 PID 的采样成功率"""
    total = 0
    failures = 0
    start_time = time.time()
    
    while time.time() - start_time < duration:
        result = subprocess.run(
            ["probing", "-t", str(pid), "backtrace"],
            capture_output=True,
            timeout=10
        )
        
        total += 1
        if result.returncode != 0:
            failures += 1
            print(f"⚠ Sample failed: {result.stderr}")
        
        time.sleep(interval)
    
    success_rate = ((total - failures) / total) * 100
    print(f"\n监控结果:")
    print(f"  总采样: {total}")
    print(f"  失败: {failures}")
    print(f"  成功率: {success_rate:.2f}%")
    
    if success_rate < 95:
        print("\n⚠ 警告: 成功率低于 95%，建议考虑迁移!")
        return 1
    return 0

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("用法: python monitor.py <pid>")
        sys.exit(1)
    
    sys.exit(monitor_sampling(int(sys.argv[1])))
```

使用:

```bash
# 启动要监控的进程
PROBING=1 python your_app.py &
APP_PID=$!

# 监控 5 分钟
python monitor.py $APP_PID --duration 300 --interval 5
```

#### 3. 设置采样频率限制

通过环境变量限制采样频率:

```bash
# 限制每分钟最多采样 10 次
export PROBING_MAX_SAMPLES_PER_MINUTE=10

# 在代码中实现速率限制
# (需要修改 stack_tracer.rs 添加此功能)
```

---

## 测试和验证 (Testing and Validation)

### 压力测试

#### 测试1: 高并发采样

```python
#!/usr/bin/env python3
"""并发采样压力测试"""
import subprocess
import multiprocessing
import time

def sample_worker(pid, count):
    """Worker 进程执行采样"""
    for i in range(count):
        subprocess.run(
            ["probing", "-t", str(pid), "backtrace"],
            capture_output=True,
            timeout=5
        )

def concurrent_sampling_test(pid, workers=10, samples_per_worker=50):
    """并发采样测试"""
    print(f"启动 {workers} 个 worker，每个执行 {samples_per_worker} 次采样...")
    
    start = time.time()
    processes = []
    
    for _ in range(workers):
        p = multiprocessing.Process(
            target=sample_worker,
            args=(pid, samples_per_worker)
        )
        p.start()
        processes.append(p)
    
    for p in processes:
        p.join()
    
    elapsed = time.time() - start
    total_samples = workers * samples_per_worker
    print(f"完成 {total_samples} 次采样，耗时 {elapsed:.2f}s")
    print(f"采样速率: {total_samples/elapsed:.2f} samples/s")

if __name__ == "__main__":
    import sys
    if len(sys.argv) < 2:
        print("用法: python stress_test.py <pid>")
        sys.exit(1)
    concurrent_sampling_test(int(sys.argv[1]))
```

#### 测试2: 长时间稳定性测试

```bash
#!/bin/bash
# 24 小时稳定性测试

PID=$1
DURATION=86400  # 24 hours
INTERVAL=60     # 每分钟采样一次

echo "开始 24 小时稳定性测试..."
echo "PID: $PID"

start_time=$(date +%s)
success_count=0
failure_count=0

while [ $(($(date +%s) - start_time)) -lt $DURATION ]; do
    if probing -t $PID backtrace > /dev/null 2>&1; then
        ((success_count++))
    else
        ((failure_count++))
        echo "$(date): 采样失败" >> stability_test.log
    fi
    
    sleep $INTERVAL
done

echo "测试完成!"
echo "成功: $success_count"
echo "失败: $failure_count"
echo "成功率: $(echo "scale=2; $success_count * 100 / ($success_count + $failure_count)" | bc)%"
```

### 回归测试

运行完整的测试套件:

```bash
# 运行所有测试
cargo test --workspace

# 运行信号处理器特定测试
python tests/test_signal_handler_safety.py

# 运行集成测试
pytest tests/ -v
```

---

## 回滚计划 (Rollback Plan)

### 情况1: pprof 迁移回滚

如果 pprof 出现问题:

```bash
# 步骤 1: 停止使用 pprof
unset PROBING_USE_PPROF

# 步骤 2: 重启应用
# (根据你的部署方式)

# 步骤 3: 验证
probing -t <pid> backtrace
```

### 情况2: 安全处理器回滚

如果安全处理器有问题:

```bash
# 步骤 1: 恢复 setup.rs
cd /home/runner/work/probing/probing
git checkout HEAD~1 -- probing/extensions/python/src/setup.rs

# 步骤 2: 移除帧指针配置
rm .cargo/config.toml

# 步骤 3: 重新编译
cargo clean
cargo build --release

# 步骤 4: 重新构建 wheel
python make_wheel.py

# 步骤 5: 重新安装
pip install --force-reinstall dist/probing-*.whl
```

### 回滚验证清单

- [ ] 编译成功
- [ ] 测试通过
- [ ] 基本功能正常
- [ ] 性能恢复正常
- [ ] 无新的错误或警告

---

## 性能对比 (Performance Comparison)

### 基准测试脚本

```python
#!/usr/bin/env python3
"""性能对比基准测试"""
import time
import subprocess
import statistics

def benchmark_backend(backend_name, env_vars, iterations=100):
    """测试特定后端的性能"""
    print(f"\n测试 {backend_name}...")
    
    script = """
import time
for i in range(1000):
    sum([x*x for x in range(1000)])
"""
    
    proc = subprocess.Popen(
        ["python", "-c", script],
        env={**env_vars, "PROBING": "1"},
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    
    time.sleep(0.1)  # 等待启动
    pid = proc.pid
    
    latencies = []
    for _ in range(iterations):
        start = time.time()
        result = subprocess.run(
            ["probing", "-t", str(pid), "backtrace"],
            capture_output=True,
            timeout=5
        )
        latency = time.time() - start
        if result.returncode == 0:
            latencies.append(latency)
    
    proc.terminate()
    proc.wait()
    
    if latencies:
        print(f"  平均延迟: {statistics.mean(latencies)*1000:.2f}ms")
        print(f"  中位数: {statistics.median(latencies)*1000:.2f}ms")
        print(f"  P95: {statistics.quantiles(latencies, n=20)[18]*1000:.2f}ms")
        print(f"  P99: {statistics.quantiles(latencies, n=100)[98]*1000:.2f}ms")
        print(f"  成功率: {len(latencies)/iterations*100:.1f}%")
    
    return latencies

if __name__ == "__main__":
    import os
    
    print("=== Probing 性能对比测试 ===")
    
    # 测试默认实现
    default = benchmark_backend("默认 SignalTracer", {})
    
    # 测试 pprof (如果已启用)
    pprof = benchmark_backend("pprof", {"PROBING_USE_PPROF": "1"})
    
    # 对比
    if default and pprof:
        overhead = (statistics.mean(pprof) / statistics.mean(default) - 1) * 100
        print(f"\npprof 相对开销: {overhead:+.1f}%")
```

运行:

```bash
python performance_benchmark.py
```

### 预期性能指标

| 后端 | 平均延迟 | P99 延迟 | CPU 开销 | 内存开销 |
|------|---------|---------|---------|---------|
| 默认 SignalTracer | 5-10ms | 20-30ms | < 1% | < 10MB |
| pprof | 3-8ms | 15-25ms | < 1% | < 20MB |
| 安全处理器 | 4-9ms | 18-28ms | < 1% | < 10MB |

---

## 常见问题 (FAQ)

### Q1: 迁移会导致停机吗？

**A**: 不会。所有迁移路线都支持热更新:
- 路线A (pprof): 通过环境变量控制，重启即可
- 路线B: 需要重新编译和安装，但可以金丝雀部署
- 路线C: 无需任何更改

### Q2: 如何选择合适的迁移路线？

**A**: 参考决策树:

```
是否是生产环境？
├─ 是 → 是否需要持续profiling？
│      ├─ 是 → 路线A (pprof) ✓
│      └─ 否 → 路线C (保持现状) + 监控
└─ 否 → 是否需要最高安全性？
       ├─ 是 → 路线B (安全处理器)
       └─ 否 → 路线C (保持现状)
```

### Q3: pprof 和 SignalTracer 可以同时使用吗？

**A**: 不推荐。它们都使用 SIGUSR2 信号，同时使用会冲突。建议选择其一。

### Q4: 迁移后性能会变差吗？

**A**: 通常不会。根据基准测试:
- pprof 可能更快（优化的采样逻辑）
- 安全处理器性能相近（符号解析延迟）

### Q5: 如果迁移失败怎么办？

**A**: 按照[回滚计划](#回滚计划)操作，通常 < 5 分钟可以回滚。

---

## 支持和反馈

如果在迁移过程中遇到问题:

1. 查看详细文档: `docs/signal-handler-safety.md`
2. 运行诊断脚本: `python tests/test_signal_handler_safety.py`
3. 提交 Issue: 包含错误日志、环境信息、复现步骤
4. 回滚到稳定版本

---

## 附录: 自动化迁移脚本

### 一键迁移到 pprof

```bash
#!/bin/bash
# migrate_to_pprof.sh - 自动迁移到 pprof

set -e

echo "=== Probing 迁移到 pprof ==="

# 1. 备份当前配置
echo "1. 备份配置..."
cp -r ~/.probing ~/.probing.backup.$(date +%s) 2>/dev/null || true

# 2. 创建配置
echo "2. 创建 pprof 配置..."
mkdir -p ~/.probing
cat > ~/.probing/config.toml << EOF
[profiling]
backend = "pprof"
sample_rate = 100
EOF

# 3. 设置环境变量
echo "3. 设置环境变量..."
echo 'export PROBING_USE_PPROF=1' >> ~/.bashrc
export PROBING_USE_PPROF=1

# 4. 测试
echo "4. 运行测试..."
python tests/test_signal_handler_safety.py || {
    echo "❌ 测试失败，回滚..."
    rm ~/.probing/config.toml
    unset PROBING_USE_PPROF
    exit 1
}

echo "✅ 迁移成功!"
echo "请重启你的应用以使用 pprof"
```

使用:

```bash
chmod +x migrate_to_pprof.sh
./migrate_to_pprof.sh
```

---

**文档版本**: 1.0  
**最后更新**: 2025-11-05  
**维护者**: Probing Team
