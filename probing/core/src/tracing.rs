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

pub fn attr<K: Into<String>, V: Into<Ele>>(key: K, value: V) -> Attribute {
    Attribute(key.into(), value.into())
}

#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    KnownLocation(u64),
    UnknownLocation(String),
}

#[derive(Debug, Clone, PartialEq)]
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
    pub fn start_span<N: Into<String>>(
        &mut self,
        name: N,
        kind: Option<&str>,
        code_path: Option<&str>,
        initial_attributes: Option<Vec<Attribute>>,
    ) -> (SpanId, TraceId) {
        let name = name.into();
        let kind = kind.map(|k_val| k_val.into());

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

        let location = code_path.map(|cp_val| Location::UnknownLocation(cp_val.into()));

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

    pub fn add_attr<S: Into<String>, V: Into<Ele>>(&mut self, key: S, value: V) {
        if let Some(active_span_id) = self.span_stack.last() {
            if let Some(span) = self.spans.get_mut(active_span_id) {
                if span.end_time.is_none() {
                    // Only add attributes to open spans
                    span.attributes
                        .get_or_insert_with(Vec::new)
                        .push(attr(key, value));
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

    pub fn add_event<S: Into<String>>(&mut self, name: S, attributes: Option<Vec<Attribute>>) {
        let name = name.into();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration; // Renamed to avoid conflict with super::Duration

    fn setup_tracer() -> LocalTracer {
        LocalTracer {
            thread_id: std::thread::current().id(),
            tracer_id: 0,
            next_trace_seq: 0,
            next_span_seq: 0,
            span_stack: vec![],
            spans: Default::default(),
        }
    }

    // --- 1. Basic Span Functionality ---

    #[test]
    fn test_start_root_span() {
        let mut tracer = setup_tracer();
        // Store tracer_id before any spans are created to correctly predict trace_id
        let current_tracer_id = tracer.tracer_id;

        let (span_id, trace_id) = tracer.start_span("root_span", None, None, None);

        assert_eq!(tracer.next_span_seq, 1);

        let span = tracer.spans.get(&span_id).expect("Span not found");
        assert_eq!(span.name, "root_span");
        assert_eq!(span.kind, None);
        assert_eq!(span.parent_span_id, None);
        assert_eq!(span.status, SpanStatus::Running);

        // TraceId should have the tracer_id as prefix and sequence number 0
        assert_eq!(
            trace_id.0,
            (current_tracer_id as u128) << TRACE_ID_PREFIX_SHIFT
        );
    }

    #[test]
    fn test_start_child_span() {
        let mut tracer = setup_tracer();
        let (root_span_id, root_trace_id) = tracer.start_span("root_span", None, None, None);

        let (child_span_id, child_trace_id) = tracer.start_span("child_span", None, None, None);

        assert_eq!(tracer.next_span_seq, 2); // Span seq 0 for root, 1 for child

        let child_span = tracer
            .spans
            .get(&child_span_id)
            .expect("Child span not found");
        assert_eq!(child_span.name, "child_span");
        assert_eq!(child_span.parent_span_id, Some(root_span_id));
        assert_eq!(child_span.status, SpanStatus::Running);

        // Child span should have the same trace_id as the root span
        assert_eq!(child_trace_id, root_trace_id);
    }

    #[test]
    fn test_end_span_behavior() {
        let mut tracer = setup_tracer();

        // 1. End a single root span
        let (root_span_id, _) = tracer.start_span("root_to_end", None, None, None);
        assert_eq!(tracer.span_stack.len(), 1);
        assert_eq!(tracer.active_id(), Some(root_span_id));

        tracer.end_span(SpanStatus::Close);
        assert!(tracer.span_stack.is_empty(), "Span stack should be empty after ending root span");
        assert_eq!(tracer.active_id(), None);

        let ended_root_span = tracer.spans.get(&root_span_id).expect("Root span not found");
        assert!(ended_root_span.end_time.is_some(), "End time should be set for root span");
        assert_eq!(ended_root_span.status, SpanStatus::Close, "Status should be Close for root span");

        // 2. End a child span, check parent status update
        let (parent_id, _) = tracer.start_span("parent", None, None, None);
        let (child_id, _) = tracer.start_span("child", None, None, None);

        assert_eq!(tracer.span_stack.len(), 2);
        let parent_span_before_child_end = tracer.spans.get(&parent_id).expect("Parent span not found");
        assert_eq!(parent_span_before_child_end.status, SpanStatus::Open, "Parent should be Open when child is Running");

        tracer.end_span(SpanStatus::Error(Some("Test error".to_string())));
        assert_eq!(tracer.span_stack.len(), 1, "Span stack should have 1 (parent) after ending child");
        assert_eq!(tracer.active_id(), Some(parent_id));

        let ended_child_span = tracer.spans.get(&child_id).expect("Child span not found");
        assert!(ended_child_span.end_time.is_some(), "End time should be set for child span");
        assert_eq!(ended_child_span.status, SpanStatus::Error(Some("Test error".to_string())), "Status should be Error for child span");

        let parent_span_after_child_end = tracer.spans.get(&parent_id).expect("Parent span not found");
        assert_eq!(parent_span_after_child_end.status, SpanStatus::Running, "Parent should be Running after child ends");

        // 3. End the parent span
        tracer.end_span(SpanStatus::Close);
        assert!(tracer.span_stack.is_empty(), "Span stack should be empty after ending parent span");
        let ended_parent_span = tracer.spans.get(&parent_id).expect("Parent span not found");
        assert!(ended_parent_span.end_time.is_some(), "End time should be set for parent span");
        assert_eq!(ended_parent_span.status, SpanStatus::Close, "Status should be Close for parent span");


        // 4. Attempt to end span when stack is empty (should not panic, logs error)
        // Reset tracer to ensure clean state for this part of the test
        let mut tracer_empty_stack = setup_tracer();
        assert!(tracer_empty_stack.span_stack.is_empty());
        tracer_empty_stack.end_span(SpanStatus::Close); // This should print an error but not panic
        // No direct assert here other than it doesn\'t panic, as the behavior is an eprintln.
    }

    #[test]
    fn test_location_unknown() {
        let mut tracer = setup_tracer();
        let code_path_str = "my_module::my_function";
        let (span_id, _) = tracer.start_span("span_with_location", None, Some(code_path_str), None);

        let span = tracer.spans.get(&span_id).unwrap();
        match &span.location {
            Some(Location::UnknownLocation(path)) => {
                assert_eq!(path, code_path_str, "Location path mismatch")
            }
            _ => panic!("Expected Some(Location::UnknownLocation)"),
        }
        tracer.end_span(SpanStatus::Close);
    }

    #[test]
    fn test_trace_id_generation_and_rollover() {
        let mut tracer = setup_tracer();
        let tracer_id_val = tracer.tracer_id as u128;

        // First trace
        let (_, trace_id1) = tracer.start_span("span1", None, None, None);
        assert_eq!(
            trace_id1.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | 0,
            "Trace ID for first trace"
        );
        tracer.end_span(SpanStatus::Close);

        // Second trace
        let (_, trace_id2) = tracer.start_span("span2", None, None, None);
        assert_eq!(
            trace_id2.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | 1,
            "Trace ID for second trace"
        );
        tracer.end_span(SpanStatus::Close);

        // Force next_trace_seq to max value
        tracer.next_trace_seq = u64::MAX - 1;

        let (_, trace_id_before_wrap) = tracer.start_span("span_before_u64_wrap", None, None, None);
        // The sequence part of trace_id is (self.next_trace_seq as u128 & MAX_TRACE_SEQ)
        assert_eq!(
            trace_id_before_wrap.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | (u64::MAX as u128 & MAX_TRACE_SEQ),
            "Trace ID before u64 wrap"
        );
        tracer.end_span(SpanStatus::Close);

        let (_, trace_id_wrap) = tracer.start_span("span_u64_wrap", None, None, None);
        assert_eq!(
            trace_id_wrap.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | (u64::MAX as u128 & MAX_TRACE_SEQ),
            "Trace ID at u64 wrap"
        );
        tracer.end_span(SpanStatus::Close);

        // next_trace_seq (u64) has now wrapped to 0.
        let (_, trace_id_after_wrap) = tracer.start_span("span_after_u64_wrap", None, None, None);
        assert_eq!(
            trace_id_after_wrap.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | (0u128 & MAX_TRACE_SEQ),
            "Trace ID after u64 wrap"
        );
        tracer.end_span(SpanStatus::Close);
    }

    #[test]
    fn test_span_id_generation_wrapping() {
        let mut tracer = setup_tracer();
        tracer.next_span_seq = u64::MAX; // Set next_span_seq to its maximum

        let (span_id1, _) = tracer.start_span("span_max_seq", None, None, None);
        assert_eq!(span_id1.0, u64::MAX, "Span ID should be u64::MAX");

        let (span_id2, _) = tracer.start_span("span_after_wrap", None, None, None);
        assert_eq!(span_id2.0, 0, "Span ID should be 0 after wrap");
    }

    // --- 2. Attribute and Event Functionality ---

    #[test]
    fn test_add_attribute_and_event() {
        let mut tracer = setup_tracer();
        let (span_id, _) = tracer.start_span("attr_event_span", None, None, None);

        // Test adding various types as attribute values
        tracer.add_attr("key_str_literal", "value1");
        tracer.add_attr("key_string_obj", "value1_string".to_string());
        tracer.add_attr("key_i32", 123i32);
        tracer.add_attr("key_i64", 456i64);
        tracer.add_attr("key_f32", 3.14f32);
        tracer.add_attr("key_f64", 2.718f64);
        tracer.add_attr("key_ele_explicit", Ele::Text("explicit_ele".to_string())); // Still works

        tracer.add_event("event1", Some(vec![attr("attr_key_in_event", 789i32)]));

        let span = tracer.spans.get(&span_id).expect("Span not found");
        let attributes = span.attributes.as_ref().expect("Attributes should exist");
        assert_eq!(attributes.len(), 7, "Expected 7 attributes");

        assert_eq!(attributes[0], attr("key_str_literal", "value1"));
        assert_eq!(
            attributes[1],
            attr("key_string_obj", "value1_string".to_string())
        );
        assert_eq!(attributes[2], attr("key_i32", 123i32));
        assert_eq!(attributes[3], attr("key_i64", 456i64));
        assert_eq!(attributes[4], attr("key_f32", 3.14f32));
        assert_eq!(attributes[5], attr("key_f64", 2.718f64));
        assert_eq!(
            attributes[6],
            attr("key_ele_explicit", Ele::Text("explicit_ele".to_string()))
        );

        assert_eq!(span.events.len(), 1);
        assert_eq!(span.events[0].name, Some("event1".to_string()));
        assert!(span.events[0].timestamp.0 > 0); // Event timestamp should be set
        assert_eq!(
            span.events[0].attributes.as_ref().unwrap()[0],
            attr("attr_key_in_event", 789i32)
        );

        // Ending the span
        tracer.end_span(SpanStatus::Close);

        // After ending the span, new attributes and events should not be added
        let span_state_before_add_after_close =
            tracer.spans.get(&span_id).expect("Span not found").clone();
        let attributes_count_before_add_after_close = span_state_before_add_after_close
            .attributes
            .as_ref()
            .map_or(0, |a| a.len());
        let events_len_before_add_after_close = span_state_before_add_after_close.events.len();

        tracer.add_attr("key_after_close_str", "value_after_close");
        tracer.add_attr("key_after_close_int", 999i32);

        let span_after_close_attempt = tracer.spans.get(&span_id).expect("Span not found");
        assert_eq!(
            span_after_close_attempt
                .attributes
                .as_ref()
                .map_or(0, |a| a.len()),
            attributes_count_before_add_after_close,
            "Attributes should not be added to a closed span"
        );
        assert_eq!(
            span_after_close_attempt.events.len(),
            events_len_before_add_after_close,
            "Events should not change for a closed span on add_attr"
        );
    }

    #[test]
    fn test_add_attribute_and_event_no_active_span() {
        let mut tracer = setup_tracer();
        // No active span
        tracer.add_attr("key_no_span_str", "val_no_span_str");
        tracer.add_attr("key_no_span_int", 0i32);
        tracer.add_event("event_no_span", None);
        // These operations print to stderr in the current code, test ensures no panic
    }
    
    // --- 3. Span Management Functionality ---

    #[test]
    fn test_active_spans_and_all_spans() {
        let mut tracer = setup_tracer();

        assert!(
            tracer.active_spans().is_empty(),
            "Active spans should be empty initially"
        );
        assert!(
            tracer.all_spans().is_empty(),
            "All spans should be empty initially"
        );

        let (s1_id, _) = tracer.start_span("s1", None, None, None);
        let (s2_id, _) = tracer.start_span("s2", None, None, None);

        let active = tracer.active_spans();
        assert_eq!(active.len(), 2, "There should be 2 active spans");
        assert!(
            active.iter().any(|s| s.span_id == s1_id),
            "s1 should be active"
        );
        assert!(
            active.iter().any(|s| s.span_id == s2_id),
            "s2 should be active"
        );

        let all = tracer.all_spans();
        assert_eq!(all.len(), 2, "There should be 2 total spans");
    }
}
