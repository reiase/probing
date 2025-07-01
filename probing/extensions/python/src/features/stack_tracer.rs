use std::sync::mpsc;
use std::sync::Mutex;

use cpp_demangle::Symbol;
use once_cell::sync::Lazy;

use probing_proto::prelude::CallFrame;

pub static NATIVE_CALLSTACK_SENDER_SLOT: Lazy<Mutex<Option<mpsc::Sender<Vec<CallFrame>>>>> =
    Lazy::new(|| Mutex::new(None));

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
                    Symbol::new(raw_name)
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

// Helper function to attempt sending frames, returns true on success, false on failure.
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

pub fn backtrace_signal_handler() {
    let native_stacks = get_native_stacks().unwrap_or_default();

    if !try_send_native_frames_to_channel(native_stacks, "native stacks (initial send)") {
        log::error!("Signal handler: CRITICAL - Failed to send native stacks. Receiver might timeout or get incomplete data.");
    }
}
