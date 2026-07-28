use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Local;
use lazy_static::lazy_static;
use nix::libc;

use probing_proto::prelude::CallFrame;

use super::super::extensions::python::get_python_stacks;
use super::snapshot_unwind::capture_raw_frames;

#[cfg(probing_snapshot_unwind)]
pub use super::snapshot_unwind::backtrace_signal_handler;

#[async_trait]
pub trait StackTracer: Send + Sync + std::fmt::Debug {
    fn trace(&self, tid: Option<i32>) -> Result<Vec<CallFrame>>;
}

#[derive(Debug)]
pub struct SignalTracer;

impl SignalTracer {
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
            match get_merge_strategy(&frame) {
                MergeType::Ignore => {}
                MergeType::MergeNativeFrame => merged.push(frame),
                MergeType::MergePythonFrame => {
                    if let Some(py_frame) = python_stacks.get(python_frame_index) {
                        merged.push(py_frame.clone());
                    }
                    python_frame_index += 1;
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

        let pid = nix::unistd::getpid().as_raw();
        let tid = tid.unwrap_or(pid);
        let native_raw_frames = capture_raw_frames(pid, tid, 2000)?;
        let native_frames = resolve_frames(native_raw_frames);

        match get_python_stacks(tid) {
            Some(python_frames) => Ok(Self::merge_python_native_stacks(
                python_frames,
                native_frames,
            )),
            None => {
                log::warn!(
                    "Failed to get Python stacks for tid {tid}; falling back to native-only stack"
                );
                Ok(native_frames)
            }
        }
    }
}

/// Resolve raw frame addresses after the signal handler has returned.
fn resolve_frames(raw_frames: Vec<*mut libc::c_void>) -> Vec<CallFrame> {
    let mut frames = vec![];

    for frame_ptr in raw_frames {
        let ip = frame_ptr;
        let symbol_address = frame_ptr;

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
    }

    frames
}

fn get_native_stacks() -> Option<Vec<CallFrame>> {
    let mut frames = vec![];
    backtrace::trace(|frame| {
        let ip = frame.ip();
        let symbol_address = frame.symbol_address();
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

pub fn exit_signal_handler(signum: i32) {
    let pid = nix::unistd::getpid().as_raw();
    let rank = env::var("RANK").unwrap_or_else(|_| "unknown".to_string());

    let cpp_frames = get_native_stacks().unwrap_or_default();
    let merged_frames = match get_python_stacks(pid) {
        Some(python_frames) => SignalTracer::merge_python_native_stacks(python_frames, cpp_frames),
        None => {
            log::warn!(
                "Failed to get Python stacks for pid {pid}; falling back to native-only stack"
            );
            cpp_frames
        }
    };

    let merged_str = serde_json::to_string_pretty(&merged_frames)
        .map_err(|error| {
            log::error!("Failed to serialize merged frames to JSON: {error}");
        })
        .unwrap_or_default();

    let log_dir = env::var("OUTPUT_DIR").unwrap_or_else(|_| "/tmp/probing_log".to_string());
    if let Err(error) = fs::create_dir_all(&log_dir) {
        log::error!("Failed to create log directory: {error}");
        return;
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M").to_string();
    let output_dir = format!("{log_dir}/output_{timestamp}");
    if let Err(error) = fs::create_dir_all(&output_dir) {
        log::error!("Failed to create output directory {output_dir}: {error}");
        return;
    }

    let merged_file_name = format!("{output_dir}/mergedstack_rank{rank}.json");
    if let Err(error) = fs::File::create(&merged_file_name)
        .and_then(|mut file| file.write_all(merged_str.as_bytes()))
    {
        log::error!("Failed to write merged stack to file {merged_file_name}: {error}");
    } else {
        let signal_name = match signum {
            nix::libc::SIGTERM => "SIGTERM(15)",
            nix::libc::SIGUSR1 => "SIGUSR1(10)",
            nix::libc::SIGABRT => "SIGABRT(6)",
            nix::libc::SIGSEGV => "SIGSEGV(11)",
            _ => "UNKNOWN SIGNAL",
        };

        println!(
            "[rank{rank}] Recieved exit signal: {signal_name}, merged stacks has been successfully written to the directory {output_dir}"
        );
    }
}

pub fn exit_signal_handler_sigterm() {
    exit_signal_handler(nix::libc::SIGTERM);
}

pub fn exit_signal_handler_sigusr1() {
    exit_signal_handler(nix::libc::SIGUSR1);
}

pub fn exit_signal_handler_sigabrt() {
    exit_signal_handler(nix::libc::SIGABRT);
}

pub extern "C" fn exit_signal_handler_sigsegv(signum: libc::c_int) {
    exit_signal_handler(signum);
    std::process::exit(1);
}
