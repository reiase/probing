mod flamegraph;

use std::collections::LinkedList;
use std::sync::Arc;
use std::sync::LazyLock;

use anyhow::Result;
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use probing_pprof::PPROF;
use probing_proto::prelude::CallFrame;
use probing_proto::types::TimeSeries;

use crate::features::spy::call::CallLocation;
use crate::features::spy::PYSTACKS;
use crate::features::stack_tracer::merge_python_native_stacks;

// static mut PPROF_CHANNEL: LazyLock<(mpsc::Sender<PProfRecord>, mpsc::Receiver<PProfRecord>)> =
//     LazyLock::new(|| mpsc::channel(100));
static PPROF_CHANNEL: Lazy<Mutex<(mpsc::Sender<PProfRecord>, mpsc::Receiver<PProfRecord>)>> = Lazy::new(|| {
    let (sender, receiver) = mpsc::channel(100);
    Mutex::new((sender, receiver))
});

pub static mut PPROF_CACHE: LazyLock<RwLock<LinkedList<PProfRecord>>> =
    LazyLock::new(|| RwLock::new(LinkedList::new()));

#[derive(Clone, Debug)]
pub struct PProfRecord {
    thread: i32,
    cframes: SmallVec<[backtrace::Frame; MAX_DEPTH]>,
    pyframes: Vec<Option<CallLocation>>,
}

impl PProfRecord {
    pub fn resolve(&self) -> Vec<CallFrame> {
        let pyframes: Vec<CallFrame> = self
            .pyframes
            .iter()
            .flatten()
            .map(|frame| CallFrame::PyFrame {
                file: frame.callee.file.clone(),
                func: frame.callee.name.clone(),
                lineno: frame.callee.line as i64,
                locals: Default::default(),
            })
            .rev()
            .collect();

        let mut ccframes: Vec<CallFrame> = vec![];
        let mut skip = true;
        self.cframes.iter().for_each(|frame| {
            backtrace::resolve_frame(frame, |symbol| {
                let ip = format!("{:X}", frame.ip() as usize);
                let frame = CallFrame::CFrame {
                    ip: ip.clone(),
                    file: symbol
                        .filename()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    func: symbol.name().map(|x| x.to_string()).unwrap_or(ip),
                    lineno: symbol.lineno().unwrap_or_default() as i64,
                };
                if skip {
                    if let CallFrame::CFrame { func, .. } = frame {
                        if func.starts_with("signal_hook_registry::handler::") {
                            skip = false;
                        }
                    }
                } else {
                    ccframes.push(frame);
                }
            });
        });
        merge_python_native_stacks(pyframes, ccframes)
    }
}

pub const MAX_DEPTH: usize = 512;

#[allow(static_mut_refs)]
unsafe extern "C" fn pprof_handler() {
    let thread = nix::libc::gettid();
    let mut cframes: SmallVec<[backtrace::Frame; MAX_DEPTH]> = SmallVec::with_capacity(MAX_DEPTH);

    let mut index = 0;
    backtrace::trace(|frame| {
        if index < MAX_DEPTH {
            cframes.push(frame.clone());
            index += 1;
            true
        } else {
            false
        }
    });

    let pyframes = PYSTACKS
        .clone()
        .iter()
        .map(|f| f.resolve().ok())
        .collect::<Vec<_>>();
    
    let channel_result = PPROF_CHANNEL.try_lock();

    if let Ok(channel_guard) = channel_result {
        match channel_guard.0.try_send(PProfRecord {
        thread,
        cframes,
        pyframes,}) {
            Ok(_) => {
            
            }
            Err(e) => {
                eprintln!("Warning: PProf channel send failed: {}", e);
            }
        }
    } else {
        eprint!("Warning: PProf channel lock failed");
    }
}

#[allow(static_mut_refs)]
pub async fn pprof_task() {
    let mut channel = PPROF_CHANNEL.try_lock().unwrap();
    let receiver = &mut channel.1;

    log::debug!("Starting pprof task to receive records");
    let backtrace_id: u64 = 0;
    while let Some(record) = receiver.recv().await {
        log::debug!("Received pprof record: {:?}", record);
        unsafe {
            PPROF_CACHE
                .try_write()
                .map(|mut cache| {
                    cache.push_back(record.clone());
                    while cache.len() > 1000 {
                        cache.pop_front();
                    }
                })
                .unwrap_or_else(|e| log::error!("Failed to read PPROF_CACHE: {}", e));
        }
    }
}

#[allow(static_mut_refs)]
pub fn setup(freq: i64) {
    log::debug!("Setting up pprof with frequency: {}", freq);
    let mut pprof = PPROF.lock().unwrap();
    pprof.set_handler(pprof_handler);
    pprof.start(Some(freq));
}

pub fn flamegraph() -> Result<String> {
    let report = flamegraph::Report::new(0);
    Ok(report.flamegraph())
}
