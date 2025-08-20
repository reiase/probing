use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use nix::libc;
use once_cell::sync::Lazy;

use probing_proto::prelude::CallFrame;
use std::fs;
use std::env;
use std::io::Write;
use chrono::Local;

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

        let ret = unsafe { libc::syscall(libc::SYS_tgkill, pid, tid, libc::SIGUSR2) };
        if ret != 0 {
            let last_error = std::io::Error::last_os_error();
            let error_msg =
                format!("Failed to send SIGUSR2 to process {pid} (thread: {tid}): {last_error}");
            log::error!("{error_msg}");
            return Err(anyhow::anyhow!(error_msg));
        }
        let python_frames = get_python_stacks(tid);

        let python_frames = python_frames.unwrap();
        
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

pub extern "C" fn exit_segvsignal_handler(_signum: libc::c_int) {
    exit_signal_handler();
    std::process::exit(1);
}

pub fn exit_signal_handler() {    
    let pid = nix::unistd::getpid().as_raw(); // PID of the current process (thread group ID)

    // Get rank number, use "unknown" if retrieval fails
    let rank = env::var("RANK").unwrap_or_else(|_| "unknown".to_string());

    let cpp_frames = SignalTracer::get_native_stacks().unwrap_or_default();
    let python_frames = get_python_stacks(pid);
    let python_frames = python_frames.unwrap();

    let merged_frames = SignalTracer::merge_python_native_stacks(python_frames, cpp_frames);

    // Convert merged stack information to string
    let merged_str = serde_json::to_string_pretty(&merged_frames)
        .map_err(|e| {
            log::error!("Failed to serialize merged frames to JSON: {}", e);
        })
        .unwrap_or_default();

    // Prioritize using OUTPUT_DIR environment variable, fallback to default path
    let log_dir = env::var("OUTPUT_DIR").unwrap_or_else(|_| "/tmp/probing_log".to_string());
    
    // Ensure log directory exists
    if let Err(e) = fs::create_dir_all(&log_dir) {
        log::error!("Failed to create log directory: {}", e);
        return;
    }

    // Generate timestamp with date (format: YYYYMMDD_HHMMSS)
    let timestamp = Local::now().format("%Y%m%d_%H%M").to_string();
    // Create output_xxx folder under the log directory
    let output_dir = format!("{}/output_{}", log_dir, timestamp);
    if let Err(e) = fs::create_dir_all(&output_dir) {
        log::error!("Failed to create output directory {}: {}", output_dir, e);
        return;
    }

    // Write mergedstack_rank{}.json into output_xxx folder
    let merged_file_name = format!("{}/mergedstack_rank{}.json", output_dir, rank);

    // Write merged stack information to file
    if let Err(e) = fs::File::create(&merged_file_name)
        .and_then(|mut file| file.write_all(merged_str.as_bytes())) {
        log::error!("Failed to write merged stack to file {}: {}", merged_file_name, e);
    } else {
        println!(
            "[rank{}] exited signal recieved, merged stacks has been Successfully written to the directory {}", rank, output_dir
        );
    }
}

/// Define a static Mutex for the backtrace function
static BACKTRACE_MUTEX: Lazy<tokio::sync::Mutex<()>> = Lazy::new(|| tokio::sync::Mutex::new(()));

pub static NATIVE_CALLSTACK_SENDER_SLOT: Lazy<Mutex<Option<mpsc::Sender<Vec<CallFrame>>>>> =
    Lazy::new(|| Mutex::new(None));
