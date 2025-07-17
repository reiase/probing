use std::collections::HashSet;
use std::env;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use nix::libc;
use once_cell::sync::Lazy;

use probing_proto::prelude::CallFrame;

use super::super::extensions::python::get_python_stacks;

#[async_trait]
pub trait StackTracer: Send + Sync + std::fmt::Debug {
    fn trace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>>;
}

#[derive(Debug)]
pub struct SignalTracer;

impl SignalTracer {
    fn get_native_stacks() -> Option<Vec<CallFrame>> {
        let mut frames = vec![];
    
        // Read and parse the CPPSTACK_SIMPLIFY environment variable, default to 0 if invalid or not set
        let cppstack_simplify = env::var("CPPSTACK_SIMPLIFY")
            .ok()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0);
    
        backtrace::trace(|frame| {
            let ip = frame.ip();
            let symbol_address = frame.symbol_address(); // Keep as *mut c_void for formatting
            backtrace::resolve_frame(frame, |symbol| {
                let func_name = symbol
                    .name()
                    .and_then(|name| name.as_str())
                    .map(|raw_name| {
                        if cppstack_simplify == 1 {
                            // Simplify C++ function names when environment variable is set to 1
                            cpp_demangle::Symbol::new(raw_name)
                                .ok()
                                .map(|demangled| simplify_cpp_name(&demangled.to_string()))
                                .unwrap_or_else(|| simplify_cpp_name(raw_name))
                        } else {
                            // Only demangle without simplification when environment variable is not 1
                            cpp_demangle::Symbol::new(raw_name)
                                .ok()
                                .map(|demangled| demangled.to_string())
                                .unwrap_or_else(|| raw_name.to_string())
                        }
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

    fn try_send_native_frames_to_channel(frames: Vec<CallFrame>, context_msg: &str) -> bool {
        log::debug!("Attempting to send native {} frames.", frames.len());
        match NATIVE_CALLSTACK_SENDER_SLOT.try_lock() {
            Ok(guard) => {
                if let Some(sender) = guard.as_ref() {
                    if sender.send(frames).is_ok() {
                        true
                    } else {
                        log::error!("Failed to send frames for {context_msg} via channel.");
                        false
                    }
                } else {
                    log::trace!("No active callstack sender found for {context_msg}.");
                    true
                }
            }
            Err(e) => {
                log::error!("Failed to lock NATIVE_CALLSTACK_SENDER_SLOT for {context_msg}: {e}");
                false
            }
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
            let mut tokens = symbol
                .split(|c| c == '_' || c == '.')
                .filter(|s| !s.is_empty());
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


/// A simplified version of the input C++ function name.
fn simplify_cpp_name(name: &str) -> String {
    /// Represents the state of the simplification process.
    enum State {
        /// Normal state, not inside any special structure.
        Normal,
        /// Inside a template, with the nested depth as a parameter.
        Template(u32),
        /// Inside a function argument list, with the nested depth as a parameter.
        Paren(u32),
        /// Inside a lambda expression, with the nested depth as a parameter.
        Lambda(u32),
    }
    
    // Initialize the state to normal.
    let mut state = State::Normal;
    // Initialize the result string to store the simplified name.
    let mut result = String::new();
    
    // Create a peekable iterator to handle characters and avoid index issues.
    let mut chars = name.chars().peekable();
    
    // Iterate through each character in the input name.
    while let Some(c) = chars.next() {
        match state {
            State::Normal => {
                match c {
                    // Handle templates: replace template parameters with '...'.
                    '<' => {
                        result.push('<');
                        result.push_str("...");
                        state = State::Template(1);
                    }
                    // Handle function argument lists: replace arguments with '...'.
                    '(' => {
                        result.push('(');
                        result.push_str("...");
                        state = State::Paren(1);
                    }
                    // Handle lambda expressions: simplify to '{lambda...}'.
                    '[' => {
                        result.push_str("{lambda...}");
                        state = State::Lambda(1);
                    }
                    // Handle colons (including '::') without special processing.
                    ':' => {
                        result.push(c);
                        // If the next character is also a colon, add it directly.
                        if let Some(&':') = chars.peek() {
                            result.push(':');
                            chars.next();  // Consume the next colon to avoid duplicate processing.
                        }
                    }
                    // Keep other characters as they are.
                    _ => result.push(c),
                }
            }
            // Skip template content until the nesting depth reaches 0.
            State::Template(depth) => {
                if c == '<' {
                    state = State::Template(depth + 1);
                } else if c == '>' {
                    if depth == 1 {
                        result.push('>');  // Close the template.
                        state = State::Normal;
                    } else {
                        state = State::Template(depth - 1);
                    }
                }
            }
            // Skip function argument list content until the nesting depth reaches 0.
            State::Paren(depth) => {
                if c == '(' {
                    state = State::Paren(depth + 1);
                } else if c == ')' {
                    if depth == 1 {
                        result.push(')');  // Close the argument list.
                        state = State::Normal;
                    } else {
                        state = State::Paren(depth - 1);
                    }
                }
            }
            // Skip lambda expression content until the nesting depth reaches 0.
            State::Lambda(depth) => {
                if c == '[' {
                    state = State::Lambda(depth + 1);
                } else if c == ']' {
                    if depth == 1 {
                        state = State::Normal;  // Lambda has been simplified, no additional characters needed.
                    } else {
                        state = State::Lambda(depth - 1);
                    }
                }
            }
        }
    }
    
    // Clean up potentially incomplete symbols (only handle template/argument related, not '::').
    if result.ends_with("<...") {
        result.truncate(result.len() - 3);
        result.push('>');  // Complete the closing symbol.
    } else if result.ends_with("(...") {
        result.truncate(result.len() - 3);
        result.push(')');  // Complete the closing symbol.
    }
    
    result
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

        let ret = unsafe { libc::syscall(libc::SYS_tgkill, pid, tid, libc::SIGUSR2) };
        if ret != 0 {
            let last_error = std::io::Error::last_os_error();
            let error_msg =
                format!("Failed to send SIGUSR2 to process {pid} (thread: {tid}): {last_error}");
            log::error!("{error_msg}");
            return Err(anyhow::anyhow!(error_msg));
        }
        let python_frames = get_python_stacks(tid);

        let python_frames = python_frames
            .and_then(|s| {
                serde_json::from_str::<Vec<CallFrame>>(&s)
                    .map_err(|e| {
                        log::error!("Failed to deserialize Python call stacks: {e}");
                        e
                    })
                    .ok()
            })
            .unwrap_or_default();
        
        let cpp_frames = rx.recv_timeout(Duration::from_secs(2))?;

        Ok(Self::merge_python_native_stacks(python_frames, cpp_frames))
    }
}

pub fn backtrace_signal_handler() {
    let native_stacks = SignalTracer::get_native_stacks().unwrap_or_default();

    if !SignalTracer::try_send_native_frames_to_channel(
        native_stacks,
        "native stacks (initial send)",
    ) {
        log::error!("Signal handler: CRITICAL - Failed to send native stacks. Receiver might timeout or get incomplete data.");
    }
}

/// Define a static Mutex for the backtrace function
static BACKTRACE_MUTEX: Lazy<tokio::sync::Mutex<()>> = Lazy::new(|| tokio::sync::Mutex::new(()));

pub static NATIVE_CALLSTACK_SENDER_SLOT: Lazy<Mutex<Option<mpsc::Sender<Vec<CallFrame>>>>> =
    Lazy::new(|| Mutex::new(None));
