# Signal Handler Safety in Stack Collection

This module implements stack trace collection using POSIX signals (`SIGUSR2`). This document explains the implementation, risks, and safety considerations.

## Quick Summary

⚠️ **Warning**: The default signal handler (`backtrace_signal_handler`) calls `backtrace-rs` functions that are **NOT async-signal-safe**. This works reliably in most cases but can theoretically cause deadlocks or crashes under specific conditions.

For detailed explanation, see [docs/signal-handler-safety.md](../../../docs/signal-handler-safety.md)

## Why Signals Are Used

Signals provide the only reliable way to:
1. **Asynchronously interrupt** a thread without its cooperation
2. **Access thread context** - signal handler runs in the interrupted thread's context
3. **Maintain low overhead** - more efficient than alternatives like `ptrace`

## The Safety Problem

The `backtrace::trace()` function used in the signal handler performs non-async-signal-safe operations:
- **Memory allocation** (malloc/free) - can deadlock if main thread holds the heap lock
- **Lock acquisition** - can deadlock with locks held by main thread
- **I/O operations** - reads debug info files, `/proc/self/maps`, etc.
- **Complex parsing** - C++ demangling, DWARF parsing, etc.

## Current Implementation

The default implementation (`backtrace_signal_handler`) prioritizes:
- ✅ **Functionality** - Reliably captures full stack traces with symbols
- ✅ **Simplicity** - Straightforward code that's easy to understand
- ⚠️ **Pragmatic risk** - Accepts small theoretical risk for practical benefits

This is the same trade-off made by most production profilers (perf, pprof, etc.).

## Safer Alternative (Experimental)

The module also provides `backtrace_signal_handler_safe()` which:
- ✅ Only captures raw frame pointers in the signal handler (more async-signal-safe)
- ✅ Defers symbol resolution to outside the handler
- ✅ Minimizes signal handler work

Trade-offs:
- ⚠️ Requires frame pointers: `RUSTFLAGS="-C force-frame-pointers=yes"`
- ⚠️ Platform-specific (currently x86_64 only)
- ⚠️ May miss frames from code compiled without frame pointers
- ⚠️ Currently experimental and not enabled by default

## Recommendations

### For Development/Debugging
- ✅ Use the default `SignalTracer` - it's simple and works well
- ✅ Be aware of the theoretical risks
- ✅ Use reasonable timeouts (current: 2 seconds)

### For Production Profiling
- ✅ Use the `pprof` crate for continuous profiling
- ✅ `pprof` handles many signal safety edge cases better
- ✅ Provides rich flamegraph and analysis features

### If You Need Maximum Safety
- ✅ Use `backtrace_signal_handler_safe()` (modify `setup.rs`)
- ✅ Compile with frame pointers enabled
- ⚠️ Test thoroughly on your target platform

## How to Switch to Safer Handler

⚠️ **详细迁移指南**: 参见 [docs/migration-guide.md](../../../docs/migration-guide.md)  
⚠️ **Detailed Migration Guide**: See [docs/migration-guide.md](../../../docs/migration-guide.md)

### Quick Switch (for development only)

Edit `probing/extensions/python/src/setup.rs`:

```rust
#[ctor]
fn setup() {
    register_signal_handler(
        nix::libc::SIGUSR2,
        // Replace the default handler:
        // crate::features::stack_tracer::backtrace_signal_handler,
        // With the safer one:
        crate::features::stack_tracer::backtrace_signal_handler_safer,
    );
}
```

Then compile with frame pointers:
```bash
RUSTFLAGS="-C force-frame-pointers=yes" cargo build
```

### Recommended for Production

Use pprof instead:

```bash
# Set environment variable
export PROBING_USE_PPROF=1

# Restart your application
PROBING=1 python your_app.py
```

For complete step-by-step instructions, migration strategies, testing procedures, and rollback plans, see the comprehensive [Migration Guide](../../../docs/migration-guide.md).

## Testing

The signal handler is exercised by:
- `SignalTracer::trace()` method
- Any backtrace collection via `probing -t <pid> backtrace`
- Profiling with pprof enabled

To test manually:
```bash
# Start a Python process with probing
PROBING=1 python examples/test_probing.py &
PID=$!

# Collect backtrace
probing -t $PID backtrace
```

## Known Issues

1. **Theoretical deadlock risk** - If the interrupted thread holds malloc's heap lock, the signal handler calling backtrace (which uses malloc) will deadlock
2. **Symbol resolution overhead** - Can be slow on first collection due to loading debug info
3. **Platform dependency** - Safer alternative currently only supports x86_64 Linux/macOS

## References

- [Full documentation](../../../docs/signal-handler-safety.md)
- [POSIX signal-safety](https://man7.org/linux/man-pages/man7/signal-safety.7.html)
- [backtrace-rs repository](https://github.com/rust-lang/backtrace-rs)
- [pprof-rs repository](https://github.com/tikv/pprof-rs)
