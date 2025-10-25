# AArch64 平台支持

本文档描述了在 AArch64 (ARM64) 平台上实现动态库注入的技术细节。

## 架构差异

### x86_64 vs AArch64

| 特性 | x86_64 | AArch64 |
|------|--------|---------|
| 调用约定 | System V AMD64 ABI | AAPCS64 |
| 参数寄存器 | rdi, rsi, rdx, rcx, r8, r9 | x0, x1, x2, x3, x4, x5, x6, x7 |
| 函数指针寄存器 | r9 | x8 |
| 返回寄存器 | rax | x0 |
| 栈对齐 | 16字节 | 16字节 |
| 陷阱指令 | int3 (0xcc) | brk #0 (0x00, 0x00, 0x20, 0xd4) |

## Shellcode 实现

### AArch64 Shellcode 结构

```rust
const SHELLCODE_AARCH64: [u8; 16] = [
    // NOP 指令用于对齐和安全
    0x1f, 0x20, 0x03, 0xd5, // nop
    0x1f, 0x20, 0x03, 0xd5, // nop
    // 调用 x8 指向的函数
    0x00, 0x01, 0x3f, 0xd6, // blr x8
    // 陷阱指令供 tracer 处理
    0x00, 0x00, 0x20, 0xd4, // brk #0
];
```

### 指令说明

1. **NOP 指令** (`0x1f, 0x20, 0x03, 0xd5`): 用于对齐和提供安全边界
2. **BLR x8** (`0x00, 0x01, 0x3f, 0xd6`): 间接分支链接指令，调用 x8 寄存器指向的函数
3. **BRK #0** (`0x00, 0x00, 0x20, 0xd4`): 断点指令，用于触发 tracer 处理

## 寄存器使用

### 函数调用约定

- **x0**: 第一个参数 / 返回值
- **x1**: 第二个参数
- **x2**: 第三个参数  
- **x3**: 第四个参数
- **x8**: 函数地址
- **sp**: 栈指针（必须16字节对齐）

### 实现示例

```rust
fn call_function(&mut self, fn_address: u64, x0: u64, x1: u64, x2: u64) -> Result<u64> {
    self.tracee.set_registers(pete::Registers {
        pc: self.injected_at,           // 程序计数器
        x8: fn_address,                 // 函数地址
        x0, x1, x2,                     // 参数
        sp: self.saved_registers.sp & !0xf, // 栈对齐
        ..self.saved_registers
    })?;
    // ... 执行和等待陷阱
}
```

## 平台检测

使用条件编译来支持不同架构：

```rust
#[cfg(target_arch = "x86_64")]
mod injection;

#[cfg(target_arch = "aarch64")]
mod injection_aarch64;
```

## 测试

### 单元测试

```rust
#[cfg(target_arch = "aarch64")]
#[test]
fn test_aarch64_shellcode() {
    // 验证 shellcode 结构
    assert_eq!(SHELLCODE_AARCH64.len(), 16);
    // 检查指令序列
    assert_eq!(SHELLCODE_AARCH64[8..12], [0x00, 0x01, 0x3f, 0xd6]); // blr x8
}
```

### 集成测试

```rust
#[cfg(target_arch = "aarch64")]
#[test]
fn test_aarch64_injection_basic() {
    // 启动目标进程
    let mut target = Command::new("sleep").arg("10").spawn()?;
    // 执行注入测试
    // ...
}
```

## 使用示例

```rust
use probing::inject::{Injector, Process};

// 在 AArch64 平台上自动使用 AArch64 注入实现
let proc = Process::by_pid(1234)?;
let mut injector = Injector::attach(proc)?;
injector.inject(&library_path, settings)?;
```

## 注意事项

1. **权限要求**: 需要适当的 ptrace 权限
2. **栈对齐**: 确保栈指针16字节对齐
3. **寄存器保存**: 正确保存和恢复寄存器状态
4. **错误处理**: 处理平台特定的错误情况

## 故障排除

### 常见问题

1. **权限被拒绝**: 检查 ptrace_scope 设置
2. **寄存器错误**: 验证寄存器保存/恢复
3. **栈对齐错误**: 确保 sp 寄存器16字节对齐
4. **函数调用失败**: 检查参数传递和函数地址

### 调试技巧

```bash
# 启用详细日志
export PROBING_LOGLEVEL=debug

# 检查进程架构
file /proc/1234/exe

# 验证 ptrace 权限
echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope
```
