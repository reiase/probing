mod flamegraph;

use std::collections::LinkedList;
use std::fmt;
use std::sync::Arc;
use std::sync::LazyLock;

use anyhow::Result;
use lazy_static::lazy_static;
use smallvec::SmallVec;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use probing_pprof::PPROF;
use probing_proto::prelude::CallFrame;
// use probing_proto::types::TimeSeries;

use crate::features::spy::call::CallLocation;
use crate::features::spy::PYSTACKS;
use crate::features::stack_tracer::merge_python_native_stacks;

struct ChannelManager {
    sender: mpsc::Sender<PProfRecord>,
    receiver: mpsc::Receiver<PProfRecord>,
    is_closed: bool, // Check if the channel is closed
    buffer_size: usize, // Buffer size for the channel
    receiver_taken: bool,
}

impl ChannelManager {
    // Create a new ChannelManager with specified buffer size
    fn new(buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        Self {
            sender,
            receiver,
            is_closed: false,
            buffer_size,
            receiver_taken: false,
        }
    }

    // Check if the channel is healthy (not closed)
    fn is_healthy(&self) -> bool {
        !self.is_closed && !self.sender.is_closed()
    }

    // Close the channel
    fn close(&mut self) {
        self.is_closed = true;
    }

    fn take_receiver(&mut self) -> Option<mpsc::Receiver<PProfRecord>> {
        if self.receiver_taken {
            log::warn!("Receiver Is Already Taken, this is a single consumer model.");
            return None;
        }
        
        self.receiver_taken = true;
        
        // Replace the existing receiver with a new one to prevent further use
        let (_, empty_receiver) = mpsc::channel(100);
        Some(std::mem::replace(&mut self.receiver, empty_receiver))
    }
}

lazy_static! {
    static ref CHANNEL_MANAGER: Arc<Mutex<ChannelManager>> = Arc::new(Mutex::new(
        ChannelManager::new(100)
    ));
}

async fn get_sender() -> Option<mpsc::Sender<PProfRecord>> {
    let manager = CHANNEL_MANAGER.lock().await;

    if !manager.is_healthy() {
        log::warn!("Channel is not healthy, cannot get sender");
        return None;
    }

    Some(manager.sender.clone())
}

pub static mut PPROF_CACHE: LazyLock<RwLock<LinkedList<PProfRecord>>> =
    LazyLock::new(|| RwLock::new(LinkedList::new()));


pub async fn send_record(record: PProfRecord) -> Result<(), String> {
    // Asynchronously get the sender
    let sender = get_sender().await.ok_or("Channel is not healthy, cannot get sender")?;

    // Try to send the record with 3 retries
    for attempt in 1..=3 {
        match sender.try_send(record.clone()) {
            Ok(_) => {
                log::info!("[SEND] Send record successfully: {:?}", record);
                return Ok(());
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                log::warn!("Buffer is full, retry in（{}）", attempt);
                if attempt == 3 {
                    return Err("Buffer is full, retry 3 times failed".to_string());
                }
                // Wait a bit before retrying
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                return Err("Channel is closed".to_string());
            }
        }
    }

    Err("Unknown error".to_string())
}

#[derive(Clone, Debug)]
pub struct PProfRecord {
    thread: i32,
    cframes: SmallVec<[backtrace::Frame; MAX_DEPTH]>,
    pyframes: Vec<Option<CallLocation>>,
}

impl fmt::Display for PProfRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PProfRecord {{ thread: {}, cframes: {}, pyframes: {} }}", 
               self.thread, self.cframes.len(), self.pyframes.len())
    }
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
    
    let record = PProfRecord {
        thread,
        cframes,
        pyframes,
    };
    
    // Use a separate async runtime to send the record
    let _ = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(send_record(record));
}

pub async fn pprof_task() {
    let receiver = {
        let mut manager = CHANNEL_MANAGER.lock().await;
        match manager.take_receiver() {
            Some(recv) => recv,
            None => {
                log::error!("Receiver is already taken, exiting pprof_task.");
                return;
            }
        }
    };

    log::info!("Receiver start work...");
    
    let mut receiver = receiver;
    while let Some(record) = receiver.recv().await {
        log::info!("[RECEIVED]: {}", record);
        // Maybe for flamegraph??
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

    log::info!("Receiver exiting...");
    // Mark the channel as closed
    let mut manager = CHANNEL_MANAGER.lock().await;
    manager.close();
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
