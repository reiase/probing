use std::collections::HashSet;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use nix::libc;
use once_cell::sync::Lazy;

use probing_proto::prelude::CallFrame;

use crate::features::vm_tracer::get_python_stacks_raw;

#[async_trait]
pub trait StackTracer: Send + Sync + std::fmt::Debug {
    fn trace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>>;
}

/// SignalTracer - Stack tracing using POSIX signals
///
/// This tracer uses `SIGUSR2` to interrupt a target thread and collect its stack trace.
///
/// ## How It Works
///
/// 1. Send `SIGUSR2` signal to target thread using `tgkill` syscall
/// 2. Signal handler (`backtrace_signal_handler`) runs in target thread's context
/// 3. Handler collects native and Python stack frames
/// 4. Frames are sent back via channel to the requestor
///
/// ## Safety Considerations
///
/// ⚠️ **WARNING**: The signal handler performs non-async-signal-safe operations.
/// See `backtrace_signal_handler()` documentation and `docs/signal-handler-safety.md`
/// for detailed explanation of risks and mitigations.
///
/// ## Alternative: Use pprof for Production
///
/// For production workloads with continuous profiling, consider using the `pprof`
/// crate which handles signal safety edge cases more robustly.
#[derive(Debug)]
pub struct SignalTracer;

impl SignalTracer {
    fn get_native_stacks() -> Option<Vec<CallFrame>> {
        let mut frames = vec![];
        backtrace::trace(|frame| {
            let ip = frame.ip();
            let symbol_address = frame.symbol_address(); // Keep as *mut c_void for formatting
            backtrace::resolve_frame(frame, |symbol| {
                let func_name = symbol
                    .name()
                    .and_then(|name| name.as_str())
                    .map(|raw_name| {
                        cpp_demangle::Symbol::new(raw_name)
                            .ok()
                            .map(|demangled| demangled.to_string())
                            .unwrap_or_else(|| raw_name.to_string())
                    })
                    .unwrap_or_else(|| format!("unknown@{symbol_address:p}"));

                let file_name = symbol
                    .filename()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default();

                frames.push(CallFrame::CFrame {
                    ip: format!("{ip:p}"),
                    file: file_name,
                    func: func_name,
                    lineno: symbol.lineno().unwrap_or(0) as i64,
                });
            });
            true
        });
        Some(frames)
    }

    fn send_frames(frames: Vec<CallFrame>) -> Result<()> {
        match NATIVE_CALLSTACK_SENDER_SLOT.try_lock() {
            Ok(guard) => {
                if let Some(sender) = guard.as_ref() {
                    sender.send(frames)?;
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("No sender available in channel slot"))
                }
            }
            Err(_) => Err(anyhow::anyhow!("Failed to send frames via channel")),
        }
    }

    fn merge_python_native_stacks(
        python_stacks: Vec<CallFrame>,
        native_stacks: Vec<CallFrame>,
    ) -> Vec<CallFrame> {
        let mut merged = vec![];
        let mut python_frame_index = 0;

        enum MergeType {
            Ignore,
            MergeNativeFrame,
            MergePythonFrame,
        }

        fn get_merge_strategy(frame: &CallFrame) -> MergeType {
            lazy_static! {
                static ref WHITELISTED_PREFIXES_SET: HashSet<&'static str> = {
                    const PREFIXES: &[&str] = &[
                        "time",
                        "sys",
                        "gc",
                        "os",
                        "unicode",
                        "thread",
                        "stringio",
                        "sre",
                        "PyGilState",
                        "PyThread",
                        "lock",
                    ];
                    PREFIXES.iter().cloned().collect()
                };
            }
            let symbol = match frame {
                CallFrame::CFrame { func, .. } => func,
                CallFrame::PyFrame { func, .. } => func,
            };
            let mut tokens = symbol.split(['_', '.']).filter(|s| !s.is_empty());
            match tokens.next() {
                Some("PyEval") => match tokens.next() {
                    Some("EvalFrameDefault" | "EvalFrameEx") => MergeType::MergePythonFrame,
                    _ => MergeType::Ignore,
                },
                Some(prefix) if WHITELISTED_PREFIXES_SET.contains(prefix) => {
                    MergeType::MergeNativeFrame
                }
                _ => MergeType::MergeNativeFrame,
            }
        }

        for frame in native_stacks {
            // log::debug!("Processing native frame: {:?}", frame);
            match get_merge_strategy(&frame) {
                MergeType::Ignore => {} // Do nothing
                MergeType::MergeNativeFrame => merged.push(frame),
                MergeType::MergePythonFrame => {
                    if let Some(py_frame) = python_stacks.get(python_frame_index) {
                        merged.push(py_frame.clone());
                    }
                    python_frame_index += 1; // Advance index regardless of whether a Python frame was available
                }
            }
        }
        merged
    }
}

#[async_trait]
impl StackTracer for SignalTracer {
    fn trace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>> {
        log::debug!("Collecting backtrace for TID: {tid:?}");

        let pid = nix::unistd::getpid().as_raw(); // PID of the current process (thread group ID)
        let tid = tid.unwrap_or(pid); // Target thread ID, or current process's PID if tid_param is None (signals the main thread)

        let _guard = BACKTRACE_MUTEX.try_lock().map_err(|e| {
            log::error!("Failed to acquire BACKTRACE_MUTEX: {e}");
            anyhow::anyhow!("Failed to acquire backtrace lock: {}", e)
        })?;

        let (tx, rx) = mpsc::channel::<Vec<CallFrame>>();
        NATIVE_CALLSTACK_SENDER_SLOT
            .try_lock()
            .map_err(|err| {
                log::error!("Failed to lock CALLSTACK_SENDER_SLOT: {err}");
                anyhow::anyhow!("Failed to lock call stack sender slot")
            })?
            .replace(tx);

        log::debug!("Sending SIGUSR2 signal to process {pid} (thread: {tid})");

        #[cfg(target_os = "linux")]
        let ret = unsafe { libc::syscall(libc::SYS_tgkill, pid, tid, libc::SIGUSR2) };

        #[cfg(target_os = "macos")]
        let ret = unsafe { libc::kill(tid, libc::SIGUSR2) };

        if ret != 0 {
            let last_error = std::io::Error::last_os_error();
            let error_msg =
                format!("Failed to send SIGUSR2 to process {pid} (thread: {tid}): {last_error}");
            log::error!("{error_msg}");
            return Err(anyhow::anyhow!(error_msg));
        }

        let native_frames = rx.recv_timeout(Duration::from_secs(2))?;
        let python_frames = rx.recv_timeout(Duration::from_secs(2))?;

        Ok(Self::merge_python_native_stacks(
            python_frames,
            native_frames,
        ))
    }
}

/// Signal handler for backtrace collection
///
/// ⚠️ **SAFETY WARNING**: This function is called from a signal handler context
/// and performs operations that are NOT async-signal-safe according to POSIX standards.
///
/// ## Why This Is Unsafe
///
/// This handler calls:
/// - `backtrace::trace()` - Uses malloc/free internally  
/// - `backtrace::resolve_frame()` - May acquire locks and perform I/O
/// - `cpp_demangle::Symbol::new()` - Complex string operations with allocations
/// - `log::error!()` - I/O operations
///
/// These operations can cause:
/// - **Deadlocks**: If the interrupted thread holds a lock that these functions need
/// - **Memory corruption**: Reentrancy issues with malloc/free
/// - **Crashes**: Undefined behavior from non-reentrant code
///
/// ## Why We Use Signals Anyway
///
/// Despite the risks, signals are necessary for:
/// 1. **Asynchronous interruption**: Can interrupt any thread without cooperation
/// 2. **Thread context**: Signal handler runs in the target thread's context
/// 3. **Low overhead**: More efficient than alternatives like ptrace
/// 4. **Industry standard**: Used by profilers like perf, pprof, etc.
///
/// ## Mitigation Strategies
///
/// 1. **Use pprof for production**: The `pprof` crate handles many edge cases better
/// 2. **Limit signal frequency**: Don't send signals too frequently
/// 3. **Monitor for hangs**: Implement timeouts for stack collection
/// 4. **Test thoroughly**: This works reliably in most cases but can fail under specific conditions
///
/// ## See Also
///
/// - `docs/signal-handler-safety.md` - Detailed explanation and safer alternatives
/// - POSIX signal-safety: `man 7 signal-safety`
///
/// ## Implementation Note
///
/// In practice, many production profilers (perf, pprof, etc.) make similar tradeoffs.
/// The key is understanding and documenting the risks.
pub fn backtrace_signal_handler() {
    let native_stacks = SignalTracer::get_native_stacks().unwrap_or_default();
    let python_stacks = get_python_stacks_raw();
    if SignalTracer::send_frames(native_stacks).is_err() {
        log::error!("Signal handler: CRITICAL - Failed to send native stacks. Receiver might timeout or get incomplete data.");
    }
    if SignalTracer::send_frames(python_stacks).is_err() {
        log::error!("Signal handler: CRITICAL - Failed to send Python stacks. Receiver might timeout or get incomplete data.");
    }
}

/// Define a static Mutex for the backtrace function
static BACKTRACE_MUTEX: Lazy<tokio::sync::Mutex<()>> = Lazy::new(|| tokio::sync::Mutex::new(()));

pub static NATIVE_CALLSTACK_SENDER_SLOT: Lazy<Mutex<Option<mpsc::Sender<Vec<CallFrame>>>>> =
    Lazy::new(|| Mutex::new(None));

// ============================================================================
// Safer Alternative Implementation (Experimental)
// ============================================================================
//
// This section provides a more async-signal-safe approach by capturing only
// raw frame pointers in the signal handler and deferring all symbol resolution.
//
// Benefits:
// - Minimizes signal handler work (async-signal-safer)
// - No malloc/free in signal handler
// - No symbol resolution in signal handler
//
// Trade-offs:
// - Requires unwinding via frame pointers (may not work with all code)
// - Currently experimental and not used by default
// - May miss frames if frame pointers are omitted during compilation
//
// To enable, compile with frame pointers: RUSTFLAGS="-C force-frame-pointers=yes"

const MAX_RAW_FRAMES: usize = 256;

/// Pre-allocated storage for raw instruction pointers captured in signal handler
/// 
/// SAFETY: AtomicPtr is async-signal-safe and can be used in signal handlers
static RAW_FRAME_BUFFER: [AtomicPtr<libc::c_void>; MAX_RAW_FRAMES] = 
    [const { AtomicPtr::new(std::ptr::null_mut()) }; MAX_RAW_FRAMES];

/// Number of frames captured in the buffer
static RAW_FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Capture raw frame pointers in an async-signal-safe manner
///
/// This function is suitable for use in signal handlers as it:
/// - Uses only async-signal-safe atomic operations
/// - Performs no memory allocation
/// - Performs no I/O
/// - Acquires no locks
///
/// NOTE: This requires frame pointers to be enabled during compilation.
/// Use: `RUSTFLAGS="-C force-frame-pointers=yes cargo build"`
#[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "macos")))]
unsafe fn capture_raw_frames_signal_safe() -> usize {
    let mut count = 0;
    let mut rbp: *mut libc::c_void;
    
    // Get current frame pointer (RBP register on x86_64)
    #[cfg(target_arch = "x86_64")]
    std::arch::asm!("mov {}, rbp", out(reg) rbp);
    
    // Walk the frame pointer chain
    // Each stack frame has the structure:
    // [saved RBP] <- RBP points here
    // [return address]
    // [local variables...]
    //
    // Frame layout (on x86_64):
    // rbp[0] = previous frame pointer
    // rbp[1] = return address
    while !rbp.is_null() && count < MAX_RAW_FRAMES {
        // Sanity check: ensure pointer is aligned and not obviously invalid
        if (rbp as usize) < 0x1000 || (rbp as usize) & 0x7 != 0 {
            break;
        }
        
        // Cast to pointer-to-pointer to access frame layout
        let frame_ptr = rbp as *mut *mut libc::c_void;
        
        // Get return address (one word above saved frame pointer)
        let return_addr = frame_ptr.offset(1).read();
        
        // Store in our pre-allocated buffer
        RAW_FRAME_BUFFER[count].store(return_addr, Ordering::Relaxed);
        count += 1;
        
        // Move to previous frame
        rbp = frame_ptr.read();
        
        // Prevent infinite loops
        if rbp as usize == 0 || rbp as usize == usize::MAX {
            break;
        }
    }
    
    count
}

/// Resolve raw frame pointers to CallFrame objects (call from safe context)
///
/// This function is NOT async-signal-safe and must be called outside
/// the signal handler context. It performs symbol resolution using backtrace-rs.
fn resolve_raw_frames(frame_count: usize) -> Vec<CallFrame> {
    let mut frames = Vec::new();
    
    for i in 0..frame_count {
        let ip = RAW_FRAME_BUFFER[i].load(Ordering::Relaxed);
        if ip.is_null() {
            continue;
        }
        
        // Now safe to use backtrace-rs for symbol resolution
        backtrace::resolve(ip, |symbol| {
            let func_name = symbol
                .name()
                .and_then(|name| name.as_str())
                .map(|raw_name| {
                    cpp_demangle::Symbol::new(raw_name)
                        .ok()
                        .map(|demangled| demangled.to_string())
                        .unwrap_or_else(|| raw_name.to_string())
                })
                .unwrap_or_else(|| format!("unknown@{ip:p}"));
            
            let file_name = symbol
                .filename()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            
            frames.push(CallFrame::CFrame {
                ip: format!("{ip:p}"),
                file: file_name,
                func: func_name,
                lineno: symbol.lineno().unwrap_or(0) as i64,
            });
        });
    }
    
    frames
}

/// Alternative safer signal handler (currently not used by default)
///
/// This handler is more async-signal-safe as it only captures raw frame pointers
/// without performing symbol resolution. Symbol resolution is deferred to a safe context.
///
/// To use this instead of the default handler, modify the signal registration in setup.rs
#[allow(dead_code)]
#[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "macos")))]
pub fn backtrace_signal_handler_safe() {
    // Capture raw frames using async-signal-safe method
    let count = unsafe { capture_raw_frames_signal_safe() };
    RAW_FRAME_COUNT.store(count, Ordering::Release);
    
    // Python stacks can still be collected here as they use simpler mechanisms
    let python_stacks = get_python_stacks_raw();
    
    // Send Python stacks (this part is still not fully async-signal-safe due to channel operations)
    // In a fully safe implementation, we'd also defer this
    if SignalTracer::send_frames(python_stacks).is_err() {
        // Can't even log safely here in a true async-signal-safe implementation
        // In practice, we accept this small risk
    }
}

/// Get native stacks from previously captured raw frames (safe context)
///
/// Call this after the signal handler has run to get the resolved stack frames
#[allow(dead_code)]
pub fn get_native_stacks_from_captured_frames() -> Vec<CallFrame> {
    let count = RAW_FRAME_COUNT.swap(0, Ordering::Acquire);
    if count == 0 {
        return vec![];
    }
    
    resolve_raw_frames(count)
}
