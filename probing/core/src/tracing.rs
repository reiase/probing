use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering}; // For unique tracer ID generation
use std::sync::{Arc, Mutex, RwLock, Weak}; // Weak for GlobalTracer's ref to LocalTracer
use std::thread::{self, ThreadId}; // For thread-local storage

use std::time::{Duration, SystemTime};

use probing_proto::types::Ele;

// --- Identifiers ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceId(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(u64);

// Global atomic counter for assigning unique short numeric IDs to LocalTracer instances.
static NEXT_TRACER_NUM: AtomicU16 = AtomicU16::new(0);

// Configuration for TraceId: 16 bits for tracer prefix, 112 bits for sequence number.
const TRACE_ID_PREFIX_SHIFT: u32 = 128 - 16; // 112 bits for sequence
const MAX_TRACE_SEQ: u128 = (1u128 << TRACE_ID_PREFIX_SHIFT) - 1;

// --- Timestamp ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u128);

impl Timestamp {
    pub fn now() -> Self {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or_else(
                |_| Timestamp(0), // Fallback for systems where time might be before UNIX_EPOCH
                |d| Timestamp(d.as_nanos()),
            )
    }

    pub fn duration_since(&self, earlier: Timestamp) -> Duration {
        if self.0 > earlier.0 {
            Duration::from_nanos((self.0 - earlier.0) as u64)
        } else {
            Duration::from_nanos(0) // Avoid panic if earlier is not actually earlier
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute(String, Ele);

#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    KnownLocation(u64),
    UnknownLocation(String),
}

#[derive(Debug, Clone)]
pub struct Event {
    pub name: Option<String>,
    pub location: Option<Location>,
    pub timestamp: Timestamp,
    pub attributes: Option<Vec<Attribute>>,
}

// --- Span Status ---
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanStatus {
    Running,               // This span is the currently active one on its thread.
    Open,                  // This span is active, but one of its children is currently Running.
    Close,                 // This span has completed successfully.
    Error(Option<String>), // This span has completed with an error.
}

impl Default for SpanStatus {
    fn default() -> Self {
        SpanStatus::Running
    }
}

#[derive(Debug, Clone)]
pub struct Span {
    // --- Identity & Relationship ---
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,

    pub name: String,
    pub kind: Option<String>,
    pub location: Option<Location>,

    pub start_time: Timestamp,
    pub end_time: Option<Timestamp>,

    pub attributes: Option<Vec<Attribute>>,
    pub events: Vec<Event>,

    // --- Outcome ---
    pub status: SpanStatus,
}

impl Span {
    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|et| et.duration_since(self.start_time))
    }
}

// --- ThreadLocal Span Manager ---
#[derive(Debug)]
pub struct LocalTracer {
    thread_id: ThreadId,
    tracer_id: u16, // Unique numeric ID for this tracer, used in TraceId generation.

    next_trace_seq: u64,
    next_span_seq: u64,

    span_stack: Vec<SpanId>,
    spans: HashMap<SpanId, Span>,
}

impl LocalTracer {
    fn new() -> Self {
        let short_tracer_id = NEXT_TRACER_NUM.fetch_add(1, Ordering::Relaxed);

        LocalTracer {
            thread_id: thread::current().id(),
            tracer_id: short_tracer_id,
            next_trace_seq: 0,
            next_span_seq: 0,
            span_stack: Vec::new(),
            spans: HashMap::new(),
        }
    }

    // --- Core Span Operations ---
    pub fn start_span(
        &mut self,
        name: String,
        kind: Option<String>,
        code_path: Option<String>,
        initial_attributes: Option<Vec<Attribute>>,
    ) -> (SpanId, TraceId) {
        let start_time = Timestamp::now();
        let current_span_sequence = self.next_span_seq;
        self.next_span_seq = self.next_span_seq.wrapping_add(1);
        let span_id = SpanId(current_span_sequence);

        let (trace_id_to_use, parent_span_id_to_store) =
            if let Some(active_parent_span_id) = self.span_stack.last() {
                let parent_span = self
                    .spans
                    .get_mut(active_parent_span_id)
                    .expect("Invariant violated: Active parent span not found in spans.");

                if parent_span.status == SpanStatus::Running {
                    parent_span.status = SpanStatus::Open;
                }

                (parent_span.trace_id, Some(*active_parent_span_id))
            } else {
                // No active parent span, so this is the start of a new trace.
                let current_trace_sequence = self.next_trace_seq;
                self.next_trace_seq = self.next_trace_seq.wrapping_add(1);

                let new_trace_id_val = ((self.tracer_id as u128) << TRACE_ID_PREFIX_SHIFT)
                    | (current_trace_sequence as u128 & MAX_TRACE_SEQ);
                (TraceId(new_trace_id_val), None)
            };

        let location = code_path.map(Location::UnknownLocation);

        let span = Span {
            trace_id: trace_id_to_use,
            span_id,
            parent_span_id: parent_span_id_to_store,
            name,
            kind,
            location,
            start_time,
            end_time: None,
            attributes: initial_attributes,
            events: vec![],
            status: SpanStatus::Running,
        };

        self.spans.insert(span_id, span);
        self.span_stack.push(span_id);
        (span_id, trace_id_to_use)
    }

    pub fn active_id(&self) -> Option<SpanId> {
        self.span_stack.last().copied()
    }

    pub fn add_attr(&mut self, key: String, value: Ele) {
        if let Some(active_span_id) = self.span_stack.last() {
            if let Some(span) = self.spans.get_mut(active_span_id) {
                if span.end_time.is_none() {
                    // Only add attributes to open spans
                    span.attributes
                        .get_or_insert_with(Vec::new)
                        .push(Attribute(key, value));
                }
            } else {
                eprintln!(
                    "Error: Active span_id {:?} not found in spans map during add_attr.",
                    active_span_id
                );
            }
        } else {
            eprintln!("Error: No active span to add attribute to.");
        }
    }

    pub fn add_event(&mut self, name: String, attributes: Option<Vec<Attribute>>) {
        if let Some(active_span_id) = self.span_stack.last() {
            if let Some(span) = self.spans.get_mut(active_span_id) {
                if span.end_time.is_none() {
                    // Only add events to open spans
                    span.events.push(Event {
                        name: Some(name),
                        location: None, // Consider if location should be set here
                        timestamp: Timestamp::now(),
                        attributes,
                    });
                }
            } else {
                eprintln!(
                    "Error: Active span_id {:?} not found in spans map during add_event.",
                    active_span_id
                );
            }
        } else {
            eprintln!("Error: No active span to add event to.");
        }
    }

    pub fn end_span(&mut self, final_status: SpanStatus) {
        let end_time = Timestamp::now();

        if let Some(active_id_on_stack) = self.span_stack.pop() {
            if let Some(ended_span) = self.spans.get_mut(&active_id_on_stack) {
                ended_span.end_time = Some(end_time);
                ended_span.status = final_status;
            } else {
                eprintln!(
                    "Error: Popped span_id {:?} not found in spans map during end_span.",
                    active_id_on_stack
                );
            }

            if let Some(parent_span_id_on_stack) = self.span_stack.last() {
                if let Some(parent_span) = self.spans.get_mut(parent_span_id_on_stack) {
                    if parent_span.status == SpanStatus::Open {
                        parent_span.status = SpanStatus::Running;
                    }
                }
            }
        } else {
            eprintln!("Error: Attempting to end span but span_stack is empty.");
        }
    }

    // --- Methods for GlobalTracer Access ---
    pub fn active_spans(&self) -> Vec<Span> {
        self.span_stack
            .iter()
            .filter_map(|id| self.spans.get(id).cloned())
            .collect()
    }

    pub fn all_spans(&self) -> Vec<Span> {
        self.spans.values().cloned().collect()
    }
}

// --- Thread-Local Storage Initialization ---
thread_local! {
    static LOCAL_TRACER: Arc<RwLock<LocalTracer>> = {
        let tracer = Arc::new(RwLock::new(LocalTracer::new()));
        GLOBAL_TRACER.register_tracer(thread::current().id(), Arc::downgrade(&tracer));
        tracer
    };
}

// --- Global Span Manager ---
#[derive(Debug, Default)]
pub struct GlobalTracer {
    local_tracers: Mutex<HashMap<ThreadId, Weak<RwLock<LocalTracer>>>>,
    // TODO: completed_span_exporter: Mutex<Option<Box<dyn SpanExporter + Send + Sync>>>,
}

impl GlobalTracer {
    pub fn new(/* exporter: Box<dyn SpanExporter + Send + Sync> */) -> Self {
        GlobalTracer {
            local_tracers: Mutex::new(HashMap::new()),
            // completed_span_exporter: Mutex::new(Some(exporter)),
        }
    }

    fn register_tracer(&self, thread_id: ThreadId, tracer: Weak<RwLock<LocalTracer>>) {
        match self.local_tracers.lock() {
            Ok(mut tracers) => {
                GlobalTracer::cleanup_locked_tracers(&mut *tracers);
                tracers.insert(thread_id, tracer);
            }
            Err(e) => {
                log::error!("Error acquiring lock on local_tracers: {}", e);
            }
        }
    }

    // Private helper function to cleanup tracers when the map is already locked.
    fn cleanup_locked_tracers(tracers_map: &mut HashMap<ThreadId, Weak<RwLock<LocalTracer>>>) {
        tracers_map.retain(|_tid, weak_tracer| weak_tracer.strong_count() > 0);
    }

    pub fn all_thread_spans(&self) -> HashMap<ThreadId, Vec<Span>> {
        let weak_tracers_to_process = match self.local_tracers.lock() {
            Ok(mut tracers_map) => {
                GlobalTracer::cleanup_locked_tracers(&mut *tracers_map);
                tracers_map.clone()
            }
            Err(e) => {
                log::error!("Error acquiring lock on local_tracers: {}", e);
                return HashMap::new(); // Return empty map on error
            }
        };

        let mut result = HashMap::new();
        for (tid, weak_tracer) in weak_tracers_to_process {
            if let Some(tracer_arc) = weak_tracer.upgrade() {
                // Lock individual LocalTracer; this lock is fine as it's per-tracer.
                let tracer_lock = tracer_arc.read().unwrap();
                result.insert(tid, tracer_lock.active_spans()); // active_spans() involves cloning
            }
        }
        result
    }

    pub fn thread_spans(&self, thread_id: ThreadId) -> Option<Vec<Span>> {
        let weak_tracer_option = match self.local_tracers.lock() {
            Ok(mut tracers_map) => {
                GlobalTracer::cleanup_locked_tracers(&mut *tracers_map);
                tracers_map.get(&thread_id).cloned()
            }
            Err(e) => {
                log::error!("Error acquiring lock on local_tracers: {}", e);
                return None; // Return empty map on error
            }
        };

        weak_tracer_option
            .and_then(|weak_tracer| weak_tracer.upgrade())
            .map(|tracer_arc| {
                // Lock individual LocalTracer.
                let tracer_lock = tracer_arc.read().unwrap();
                tracer_lock.active_spans() // active_spans() involves cloning
            })
    }
}

pub static GLOBAL_TRACER: Lazy<GlobalTracer> = Lazy::new(GlobalTracer::new);
