use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::hash::Hash; // Added for SpanStatus hashing
use std::sync::atomic::{AtomicU16, Ordering}; // For unique tracer ID generation
use std::sync::{Arc, Mutex, RwLock, Weak}; // Ensure PoisonError is imported
use std::thread::{self, ThreadId}; // For thread-local storage

use super::TraceError; // Import TraceError from parent module

use std::time::{Duration, SystemTime};

pub use probing_proto::types::Ele;

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
#[derive(Debug, Clone, PartialEq, Eq, Hash)] // Added Hash
#[derive(Default)]
pub enum SpanStatus {
    #[default]
    Running, // This span is the currently active one on its thread.
    Open,                  // This span is active, but one of its children is currently Running.
    Close,                 // This span has completed successfully.
    Error(Option<String>), // This span has completed with an error.
}

// --- Span Statistics --- (NEW)
/// Holds statistics for a group of spans sharing the same kind, name, and status.
#[derive(Debug, Clone, Default)]
pub struct SpanStats {
    /// The number of spans in this group.
    pub count: u64,
    /// The total duration accumulated by all spans in this group.
    pub total_duration: Duration, // std::time::Duration
}
// --- End Span Statistics --- (NEW)

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
pub struct LocalSpanManager {
    _thread_id: ThreadId,
    tracer_id: u16, // Unique numeric ID for this tracer, used in TraceId generation.

    next_trace_seq: u64,
    next_span_seq: u64,

    span_stack: Vec<SpanId>,
    spans: HashMap<SpanId, Span>,
    statistics: HashMap<(Option<String>, String, SpanStatus), SpanStats>, // Added statistics field
}

impl LocalSpanManager {
    fn new() -> Self {
        let short_tracer_id = NEXT_TRACER_NUM.fetch_add(1, Ordering::Relaxed);

        LocalSpanManager {
            _thread_id: thread::current().id(),
            tracer_id: short_tracer_id,
            next_trace_seq: 0,
            next_span_seq: 0,
            span_stack: Vec::new(),
            spans: HashMap::new(),
            statistics: HashMap::new(), // Initialize statistics
        }
    }

    // --- Core Span Operations ---
    pub fn start_span<N: Into<String>>(
        &mut self,
        name: N,
        kind: Option<&str>,
        code_path: Option<&str>,
        // initial_attributes: Option<Vec<Attribute>>, // Removed
    ) -> (SpanId, TraceId) {
        let name = name.into();
        let kind = kind.map(|k_val| k_val.into());
        // let attributes = initial_attributes; // Removed

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
            attributes: None, // Changed from initial_attributes
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
                    "Error: Active span_id {active_span_id:?} not found in spans map during add_attr."
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
                    "Error: Active span_id {active_span_id:?} not found in spans map during add_event."
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
                ended_span.status = final_status.clone(); // Update span's status

                // Update statistics
                if let Some(duration) = ended_span.duration() {
                    let key = (
                        ended_span.kind.clone(),
                        ended_span.name.clone(),
                        final_status, // Use the final_status for the key
                    );
                    let stats_entry = self.statistics.entry(key).or_default();
                    stats_entry.count += 1;
                    stats_entry.total_duration += duration;
                }
            } else {
                eprintln!(
                    "Error: Popped span_id {active_id_on_stack:?} not found in spans map during end_span."
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
    pub fn list_spans(&self) -> Vec<Span> {
        self.span_stack
            .iter()
            .filter_map(|id| self.spans.get(id).cloned())
            .collect()
    }

    pub fn current_span(&self) -> Option<Span> {
        self.active_id().and_then(|id| self.spans.get(&id).cloned())
    }

    pub fn all_spans(&self) -> Vec<Span> {
        self.spans.values().cloned().collect()
    }

    /// Retrieves a clone of the collected span statistics, including counts for active spans.
    /// Statistics are aggregated by (kind, name, status).
    /// For active spans (those currently on the stack), only their count is included;
    /// their duration is not added to `total_duration` as they are still running.
    pub fn get_statistics(&self) -> HashMap<(Option<String>, String, SpanStatus), SpanStats> {
        let mut final_stats = self.statistics.clone(); // Clone completed stats

        // Add active spans directly
        for span_id in &self.span_stack {
            if let Some(span) = self.spans.get(span_id) {
                let key = (
                    span.kind.clone(),
                    span.name.clone(),
                    span.status.clone(), // This will be Running or Open
                );
                let entry = final_stats.entry(key).or_default();
                entry.count += 1;
                // Do not add to total_duration for active spans
            }
        }
        final_stats
    }
}

// --- Thread-Local Storage Initialization ---
thread_local! {
    pub(crate) static LOCAL_TRACER: Arc<RwLock<LocalSpanManager>> = {
        let tracer = Arc::new(RwLock::new(LocalSpanManager::new()));
        GLOBAL_TRACER.register_tracer(thread::current().id(), Arc::downgrade(&tracer));
        tracer
    };
}

// --- Global Span Manager ---
#[derive(Debug, Default)]
pub struct GlobalSpanManager {
    local_tracers: Mutex<HashMap<ThreadId, Weak<RwLock<LocalSpanManager>>>>,
    // TODO: completed_span_exporter: Mutex<Option<Box<dyn SpanExporter + Send + Sync>>>;,
}

impl GlobalSpanManager {
    pub fn new(/* exporter: Box<dyn SpanExporter + Send + Sync> */) -> Self {
        GlobalSpanManager {
            local_tracers: Mutex::new(HashMap::new()),
            // completed_span_exporter: Mutex::new(Some(exporter)),
        }
    }

    fn register_tracer(&self, thread_id: ThreadId, tracer: Weak<RwLock<LocalSpanManager>>) {
        match self.local_tracers.lock() {
            Ok(mut tracers) => {
                GlobalSpanManager::cleanup_locked_tracers(&mut tracers);
                tracers.insert(thread_id, tracer);
            }
            Err(e) => {
                log::error!("Failed to lock local_tracers for registration: {e}");
            }
        }
    }

    fn cleanup_locked_tracers(tracers_map: &mut HashMap<ThreadId, Weak<RwLock<LocalSpanManager>>>) {
        tracers_map.retain(|_tid, weak_tracer| weak_tracer.strong_count() > 0);
    }

    pub fn all_thread_spans(&self) -> Result<HashMap<ThreadId, Vec<Span>>, TraceError> {
        let mut tracers_map_guard = self.local_tracers.lock()?;
        GlobalSpanManager::cleanup_locked_tracers(&mut tracers_map_guard);

        let mut result = HashMap::new();
        let mut local_tracer_arcs = Vec::new();
        for (tid, weak_tracer) in tracers_map_guard.iter() {
            if let Some(tracer_arc) = weak_tracer.upgrade() {
                local_tracer_arcs.push((*tid, tracer_arc));
            }
        }

        drop(tracers_map_guard);

        for (tid, tracer_arc) in local_tracer_arcs {
            let tracer_lock = tracer_arc.read()?; // This will now correctly use From<PoisonError<T>> for TraceError
            result.insert(tid, tracer_lock.list_spans());
        }
        Ok(result)
    }

    pub fn thread_spans(&self, thread_id: ThreadId) -> Result<Option<Vec<Span>>, TraceError> {
        let mut tracers_map_guard = self.local_tracers.lock()?;
        GlobalSpanManager::cleanup_locked_tracers(&mut tracers_map_guard);

        let weak_tracer_option = tracers_map_guard.get(&thread_id).cloned();
        drop(tracers_map_guard);

        if let Some(weak_tracer) = weak_tracer_option {
            if let Some(tracer_arc) = weak_tracer.upgrade() {
                let tracer_lock = tracer_arc.read()?; // This will now correctly use From<PoisonError<T>> for TraceError
                return Ok(Some(tracer_lock.list_spans()));
            }
        }
        Ok(None)
    }
}

pub static GLOBAL_TRACER: Lazy<GlobalSpanManager> = Lazy::new(GlobalSpanManager::new);

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration; // Renamed to avoid conflict with super::Duration

    fn setup_tracer() -> LocalSpanManager {
        LocalSpanManager {
            _thread_id: std::thread::current().id(),
            tracer_id: 0, // Simplified for predictable test outcomes
            next_trace_seq: 0,
            next_span_seq: 0,
            span_stack: vec![],
            spans: Default::default(),
            statistics: Default::default(), // Initialize statistics
        }
    }

    // Helper functions to reduce test redundancy
    fn create_span_with_duration(
        tracer: &mut LocalSpanManager,
        name: &str,
        kind: Option<&str>,
        duration_ms: u64,
        status: SpanStatus,
    ) {
        tracer.start_span(name, kind, None);
        std::thread::sleep(StdDuration::from_millis(duration_ms));
        tracer.end_span(status);
    }

    fn assert_stats_entry(
        stats: &std::collections::HashMap<(Option<String>, String, SpanStatus), SpanStats>,
        key: (Option<String>, String, SpanStatus),
        expected_count: u64,
        min_duration: Option<StdDuration>,
    ) {
        let stats_entry = stats
            .get(&key)
            .unwrap_or_else(|| panic!("Stats not found for key: {key:?}"));
        assert_eq!(
            stats_entry.count, expected_count,
            "Count mismatch for key: {key:?}"
        );

        if let Some(min_dur) = min_duration {
            assert!(
                stats_entry.total_duration >= min_dur,
                "Duration too short for key: {:?}. Expected: {:?}, Got: {:?}",
                key,
                min_dur,
                stats_entry.total_duration
            );
        } else {
            assert_eq!(
                stats_entry.total_duration,
                StdDuration::from_nanos(0),
                "Expected zero duration for active span: {key:?}",
            );
        }
    }

    fn create_span_key(
        kind: Option<&str>,
        name: &str,
        status: SpanStatus,
    ) -> (Option<String>, String, SpanStatus) {
        (kind.map(String::from), name.to_string(), status)
    }

    // --- 1. Basic Span Functionality ---

    #[test]
    fn test_start_root_span_example() {
        // Example: Starting a new trace with a root span.
        // This is typically the first operation when tracing a new request or process.
        let mut tracer = setup_tracer();
        let tracer_id_for_assertion = tracer.tracer_id; // For assertion clarity

        let (span_id, trace_id) = tracer.start_span(
            "process_incoming_request", // A descriptive name for the span
            Some("server_op"),          // Optional: kind of operation (e.g., server, client)
            Some("my_app::request_handler"), // Optional: code path or function name
                                        // None,                            // initial_attributes removed
        );

        assert_eq!(
            tracer.span_stack.len(),
            1,
            "A new span should be on the stack"
        );
        assert_eq!(
            tracer.active_id(),
            Some(span_id),
            "The new span should be active"
        );

        let span = tracer
            .spans
            .get(&span_id)
            .expect("Span not found in tracer");
        assert_eq!(span.name, "process_incoming_request");
        assert_eq!(span.kind, Some("server_op".to_string()));
        assert_eq!(span.parent_span_id, None, "Root span has no parent");
        assert_eq!(
            span.status,
            SpanStatus::Running,
            "New span should be running"
        );
        match &span.location {
            Some(Location::UnknownLocation(path)) => assert_eq!(path, "my_app::request_handler"),
            _ => panic!("Expected UnknownLocation with the specified code_path"),
        }

        // Verify TraceId incorporates the tracer's ID and the trace sequence number (0 for the first trace)
        let expected_trace_id_val =
            (tracer_id_for_assertion as u128) << TRACE_ID_PREFIX_SHIFT;
        assert_eq!(
            trace_id.0, expected_trace_id_val,
            "Trace ID mismatch for root span"
        );

        tracer.end_span(SpanStatus::Close); // Remember to end your spans
    }

    #[test]
    fn test_start_child_span_example() {
        // Example: Creating a child span within an existing trace.
        // This is used to trace sub-operations of a larger task.
        let mut tracer = setup_tracer();
        let (root_span_id, root_trace_id) = tracer.start_span("root_operation", None, None); // Removed None for attributes

        // Start a child span for a sub-task
        let (child_span_id, child_trace_id) = tracer.start_span(
            "database_query",
            Some("db_client"),
            Some("my_app::db_service::query"),
            // None, // Removed None for attributes
        );

        assert_eq!(
            tracer.span_stack.len(),
            2,
            "Root and child spans should be on stack"
        );
        assert_eq!(
            tracer.active_id(),
            Some(child_span_id),
            "Child span should be active"
        );

        let child_span = tracer
            .spans
            .get(&child_span_id)
            .expect("Child span not found");
        assert_eq!(child_span.name, "database_query");
        assert_eq!(
            child_span.parent_span_id,
            Some(root_span_id),
            "Child's parent should be the root span"
        );
        assert_eq!(child_span.status, SpanStatus::Running);
        assert_eq!(
            child_trace_id, root_trace_id,
            "Child span must share the same TraceId as its root"
        );
        assert!(
            child_span.attributes.is_none(),
            "Initial attributes should now be None"
        );

        let root_span_after_child_start = tracer
            .spans
            .get(&root_span_id)
            .expect("Root span not found");
        assert_eq!(
            root_span_after_child_start.status,
            SpanStatus::Open,
            "Root span should be 'Open' as child is 'Running'"
        );

        tracer.end_span(SpanStatus::Close); // End child span
        tracer.end_span(SpanStatus::Close); // End root span
    }

    #[test]
    fn test_end_span_scenarios_example() {
        // Example: Demonstrating different ways spans are ended and how parent status is affected.
        let mut tracer = setup_tracer();

        // Scenario 1: Start and end a single root span successfully.
        let (root_span_id, _) = tracer.start_span("single_task", None, None); // Removed None for attributes
        assert_eq!(tracer.active_id(), Some(root_span_id));
        tracer.end_span(SpanStatus::Close); // Mark as successfully closed
        assert!(
            tracer.span_stack.is_empty(),
            "Stack should be empty after root span ends"
        );
        let ended_root_span = tracer.spans.get(&root_span_id).expect("Root span missing");
        assert!(ended_root_span.end_time.is_some(), "End time must be set");
        assert_eq!(ended_root_span.status, SpanStatus::Close);

        // Scenario 2: End a child span with an error, and observe parent's status change.
        let (parent_id, _) = tracer.start_span("main_operation", None, None); // Removed None for attributes
        let (child_id, _) = tracer.start_span("sub_operation_fails", None, None); // Removed None for attributes

        let parent_span_while_child_active =
            tracer.spans.get(&parent_id).expect("Parent span missing");
        assert_eq!(
            parent_span_while_child_active.status,
            SpanStatus::Open,
            "Parent is 'Open' while child is 'Running'"
        );

        let error_message = "Something went wrong".to_string();
        tracer.end_span(SpanStatus::Error(Some(error_message.clone()))); // End child with an error
        assert_eq!(
            tracer.active_id(),
            Some(parent_id),
            "Parent should be the active span now"
        );

        let ended_child_span = tracer.spans.get(&child_id).expect("Child span missing");
        assert!(ended_child_span.end_time.is_some());
        assert_eq!(
            ended_child_span.status,
            SpanStatus::Error(Some(error_message))
        );

        let parent_span_after_child_error =
            tracer.spans.get(&parent_id).expect("Parent span missing");
        assert_eq!(
            parent_span_after_child_error.status,
            SpanStatus::Running,
            "Parent becomes 'Running' again after child ends"
        );

        // Scenario 3: End the parent span successfully.
        tracer.end_span(SpanStatus::Close);
        assert!(
            tracer.span_stack.is_empty(),
            "Stack empty after parent ends"
        );
        let ended_parent_span = tracer.spans.get(&parent_id).expect("Parent span missing");
        assert!(ended_parent_span.end_time.is_some());
        assert_eq!(ended_parent_span.status, SpanStatus::Close);

        // Scenario 4: Attempting to end a span when no spans are active (e.g., programming error).
        // This should not panic the tracer but log an error (checked by observing eprintln output if testing framework allows).
        // For this example, we just ensure it doesn't crash.
        let mut fresh_tracer = setup_tracer();
        fresh_tracer.end_span(SpanStatus::Close); // No panic expected
    }

    #[test]
    fn test_span_with_location_example() {
        // Example: Starting a span with a specified code path (location).
        let mut tracer = setup_tracer();
        let module_path = "my_app::services::user_service::create_user";
        let (span_id, _) = tracer.start_span("create_user_call", None, Some(module_path)); // Removed None for attributes

        let span = tracer.spans.get(&span_id).unwrap();
        match &span.location {
            Some(Location::UnknownLocation(path)) => {
                assert_eq!(
                    path, module_path,
                    "Span location should match the provided code_path"
                );
            }
            _ => panic!("Expected span.location to be Some(Location::UnknownLocation)"),
        }
        tracer.end_span(SpanStatus::Close);
    }

    // These tests verify internal ID generation logic, less for direct user example but crucial for correctness.
    #[test]
    fn test_trace_id_generation_and_rollover() {
        let mut tracer = setup_tracer();
        let tracer_id_val = tracer.tracer_id as u128;

        // First trace
        let (_, trace_id1) = tracer.start_span("span1", None, None); // Removed None for attributes
        assert_eq!(
            trace_id1.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT),
            "Trace ID for first trace"
        );
        tracer.end_span(SpanStatus::Close);

        // Second trace
        let (_, trace_id2) = tracer.start_span("span2", None, None); // Removed None for attributes
        assert_eq!(
            trace_id2.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | 1,
            "Trace ID for second trace"
        );
        tracer.end_span(SpanStatus::Close);

        // Force next_trace_seq to max value
        tracer.next_trace_seq = u64::MAX - 1;

        let (_, trace_id_before_wrap) = tracer.start_span("span_before_u64_wrap", None, None); // Removed None for attributes
                                                                                               // The sequence part of trace_id is (self.next_trace_seq as u128 & MAX_TRACE_SEQ)
        assert_eq!(
            trace_id_before_wrap.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | ((u64::MAX - 1) as u128 & MAX_TRACE_SEQ),
            "Trace ID before u64 wrap"
        );
        tracer.end_span(SpanStatus::Close);

        let (_, trace_id_wrap) = tracer.start_span("span_u64_wrap", None, None); // Removed None for attributes
        assert_eq!(
            trace_id_wrap.0,
            (tracer_id_val << TRACE_ID_PREFIX_SHIFT) | (u64::MAX as u128 & MAX_TRACE_SEQ),
            "Trace ID at u64 wrap"
        );
        tracer.end_span(SpanStatus::Close);

        // next_trace_seq (u64) has now wrapped to 0.
        let (_, trace_id_after_wrap) = tracer.start_span("span_after_u64_wrap", None, None); // Removed None for attributes
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

        let (span_id1, _) = tracer.start_span("span_max_seq", None, None); // Removed None for attributes
        assert_eq!(span_id1.0, u64::MAX, "Span ID should be u64::MAX");

        let (span_id2, _) = tracer.start_span("span_after_wrap", None, None); // Removed None for attributes
        assert_eq!(span_id2.0, 0, "Span ID should be 0 after wrap");
    }

    // --- 2. Attribute and Event Functionality ---

    #[test]
    fn test_add_attributes_and_events_example() {
        // Example: Adding attributes and events to a span to record contextual information.
        let mut tracer = setup_tracer();
        let (span_id, _) = tracer.start_span("user_request_processing", None, None); // Removed None for attributes

        // Add attributes with various data types. Attributes provide key-value details about the span.
        tracer.add_attr("http.method", "GET");
        tracer.add_attr("http.url", "/users/123".to_string());
        tracer.add_attr("user.id", 123i32);
        tracer.add_attr("request.size_bytes", 1024i64);
        tracer.add_attr("cache.hit_ratio", 0.75f32);
        tracer.add_attr("processing.time_ms", 123.456f64);
        tracer.add_attr("custom.info", Ele::Text("important_detail".to_string()));

        // Add an event. Events are timestamped occurrences within a span, with their own attributes.
        tracer.add_event(
            "cache_lookup",
            Some(vec![
                attr("cache.key", "user_123_data"),
                attr("cache.hit", true),
            ]),
        );

        // Simulate some work
        std::thread::sleep(StdDuration::from_millis(5));

        tracer.add_event("validation_complete", None); // Event without attributes

        {
            let span = tracer.spans.get(&span_id).expect("Span not found");
            let attributes = span.attributes.as_ref().expect("Attributes should exist");
            assert_eq!(attributes.len(), 7, "Expected 7 attributes on the span");
            assert_eq!(attributes[0], attr("http.method", "GET"));
            assert_eq!(attributes[1], attr("http.url", "/users/123".to_string()));
            // ... (assertions for other attributes can be added for completeness)

            assert_eq!(span.events.len(), 2, "Expected 2 events in the span");
            assert_eq!(span.events[0].name, Some("cache_lookup".to_string()));
            assert!(
                span.events[0].timestamp.0 > span.start_time.0,
                "Event timestamp should be after span start"
            );
            let event1_attrs = span.events[0].attributes.as_ref().unwrap();
            assert_eq!(event1_attrs.len(), 2);
            assert_eq!(event1_attrs[0], attr("cache.key", "user_123_data"));
            assert_eq!(event1_attrs[1], attr("cache.hit", true));

            assert_eq!(span.events[1].name, Some("validation_complete".to_string()));
            assert!(
                span.events[1].timestamp.0 > span.events[0].timestamp.0,
                "Second event should be later"
            );
            assert!(span.events[1].attributes.is_none());
        }
        tracer.end_span(SpanStatus::Close);

        // Behavior check: Attributes and events cannot be added to a closed span.
        let span = tracer.spans.get(&span_id).expect("Span not found");
        let attributes_count_before_add_after_close =
            span.attributes.as_ref().map_or(0, |a| a.len());
        let events_len_before_add_after_close = span.events.len();

        tracer.add_attr("attempt_after_close", "should_not_be_added");
        tracer.add_event("event_after_close", None);

        let span_after_close_attempt = tracer
            .spans
            .get(&span_id)
            .expect("Span not found after close");
        assert_eq!(
            span_after_close_attempt
                .attributes
                .as_ref()
                .map_or(0, |a| a.len()),
            attributes_count_before_add_after_close,
            "Attributes should not be added to a closed span."
        );
        assert_eq!(
            span_after_close_attempt.events.len(),
            events_len_before_add_after_close,
            "Events should not be added to a closed span."
        );
    }

    #[test]
    fn test_add_attribute_and_event_to_non_existent_span_example() {
        // Example: Demonstrating behavior when trying to add attributes/events without an active span.
        // This typically indicates a misuse of the tracer or a logic error in instrumentation.
        // The tracer should handle this gracefully (e.g., by logging an error) without panicking.
        let mut tracer = setup_tracer(); // No spans started

        tracer.add_attr("key_no_span", "value_no_span"); // Should not panic, might log error
        tracer.add_event("event_no_span", None); // Should not panic, might log error

        // Assert that no spans were created or modified if that's the intended behavior for error logging.
        assert!(
            tracer.spans.is_empty(),
            "No spans should exist if none were started."
        );
    }

    // --- 3. Statistics Functionality ---

    #[test]
    fn test_get_statistics_empty() {
        let tracer = setup_tracer();
        let stats = tracer.get_statistics();
        assert!(
            stats.is_empty(),
            "Statistics should be empty for a new tracer"
        );
    }

    #[test]
    fn test_get_statistics_completed_spans() {
        let mut tracer = setup_tracer();

        // Create multiple spans with different scenarios
        let span_scenarios = [
            ("task_a", Some("type1"), 10, SpanStatus::Close),
            ("task_a", Some("type1"), 20, SpanStatus::Close),
            (
                "task_b",
                Some("type2"),
                30,
                SpanStatus::Error(Some("failed".to_string())),
            ),
        ];

        for (name, kind, duration, status) in span_scenarios {
            create_span_with_duration(&mut tracer, name, kind, duration, status);
        }

        let stats = tracer.get_statistics();
        assert_eq!(stats.len(), 2, "Expected 2 groups of statistics");

        // Verify task_a statistics (2 completed spans)
        let task_a_key = create_span_key(Some("type1"), "task_a", SpanStatus::Close);
        assert_stats_entry(&stats, task_a_key, 2, Some(StdDuration::from_millis(30)));

        // Verify task_b statistics (1 error span)
        let task_b_key = create_span_key(
            Some("type2"),
            "task_b",
            SpanStatus::Error(Some("failed".to_string())),
        );
        assert_stats_entry(&stats, task_b_key, 1, Some(StdDuration::from_millis(30)));
    }

    #[test]
    fn test_get_statistics_active_spans() {
        let mut tracer = setup_tracer();

        // Create nested active spans
        tracer.start_span("active_task_1", Some("type_active"), None);
        tracer.start_span("active_task_2_parent", Some("type_parent_active"), None);
        tracer.start_span("active_task_2_child", Some("type_child_active"), None);

        let stats = tracer.get_statistics();
        assert_eq!(stats.len(), 3, "Expected 3 groups for active spans");

        // Test all active spans have zero duration
        let active_keys = [
            create_span_key(Some("type_active"), "active_task_1", SpanStatus::Open),
            create_span_key(
                Some("type_parent_active"),
                "active_task_2_parent",
                SpanStatus::Open,
            ),
            create_span_key(
                Some("type_child_active"),
                "active_task_2_child",
                SpanStatus::Running,
            ),
        ];

        for key in active_keys {
            assert_stats_entry(&stats, key, 1, None); // None means expect zero duration
        }

        // End child and verify parent becomes Running
        tracer.end_span(SpanStatus::Close);
        let stats_after = tracer.get_statistics();

        assert_stats_entry(
            &stats_after,
            create_span_key(
                Some("type_child_active"),
                "active_task_2_child",
                SpanStatus::Close,
            ),
            1,
            Some(StdDuration::from_nanos(1)),
        );
        assert_stats_entry(
            &stats_after,
            create_span_key(
                Some("type_parent_active"),
                "active_task_2_parent",
                SpanStatus::Running,
            ),
            1,
            None,
        );
    }

    #[test]
    fn test_get_statistics_mixed_spans() {
        let mut tracer = setup_tracer();

        // Create mixed scenario: completed, active, and error spans
        create_span_with_duration(
            &mut tracer,
            "completed_task",
            Some("type_c"),
            15,
            SpanStatus::Close,
        );

        tracer.start_span("active_task", Some("type_a"), None); // This will remain active

        create_span_with_duration(
            &mut tracer,
            "active_task",
            Some("type_a"),
            25,
            SpanStatus::Error(None),
        );

        let stats = tracer.get_statistics();
        assert_eq!(stats.len(), 3, "Expected 3 groups of statistics");

        // Use helper to verify all expected statistics
        let expected_stats = [
            (
                create_span_key(Some("type_c"), "completed_task", SpanStatus::Close),
                1,
                Some(StdDuration::from_millis(15)),
            ),
            (
                create_span_key(Some("type_a"), "active_task", SpanStatus::Running),
                1,
                None,
            ),
            (
                create_span_key(Some("type_a"), "active_task", SpanStatus::Error(None)),
                1,
                Some(StdDuration::from_millis(25)),
            ),
        ];

        for (key, count, min_duration) in expected_stats {
            assert_stats_entry(&stats, key, count, min_duration);
        }
    }

    #[test]
    fn test_get_statistics_multiple_statuses_for_same_span_name_kind() {
        let mut tracer = setup_tracer();
        let span_name = "task_x";
        let span_kind = Some("type_multi");

        // Create multiple instances with different statuses
        let span_instances = [
            (5, SpanStatus::Close),
            (8, SpanStatus::Error(Some("network issue".to_string()))),
            (6, SpanStatus::Close),
        ];

        for (duration, status) in span_instances {
            create_span_with_duration(&mut tracer, span_name, span_kind, duration, status);
        }

        // Create one active span
        tracer.start_span(span_name, span_kind, None);

        let stats = tracer.get_statistics();
        assert_eq!(stats.len(), 3, "Expected 3 distinct groups based on status");

        // Verify statistics for each status type
        let expected_results = [
            (SpanStatus::Close, 2, Some(StdDuration::from_millis(11))), // 5 + 6
            (
                SpanStatus::Error(Some("network issue".to_string())),
                1,
                Some(StdDuration::from_millis(8)),
            ),
            (SpanStatus::Running, 1, None),
        ];

        for (status, count, min_duration) in expected_results {
            let key = create_span_key(span_kind, span_name, status);
            assert_stats_entry(&stats, key, count, min_duration);
        }
    }
}
