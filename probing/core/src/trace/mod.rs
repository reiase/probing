mod span;

use crate::trace::span::{Attribute, Ele, SpanStatus, GLOBAL_TRACER, LOCAL_TRACER};
use std::collections::HashMap;
use std::sync::PoisonError;
use std::thread::ThreadId;

// --- Custom Error Type ---

/// Represents errors that can occur during tracing operations.
#[derive(Debug)]
pub enum TraceError {
    /// Indicates that a Mutex or RwLock was poisoned.
    /// This typically occurs if a thread panics while holding a lock.
    LockPoisoned,
    // Potentially, other tracing-specific errors could be added here in the future.
    // For example:
    // SpanNotFound,
    // InvalidSpanOperation(String),
}

// Implement From<PoisonError> for TraceError to allow '?' to work with RwLock errors
impl<T> From<PoisonError<T>> for TraceError {
    fn from(_err: PoisonError<T>) -> Self {
        // Log the original error if a logging facade is available
        // log::error!("Lock poisoned: {:?}", err);
        TraceError::LockPoisoned
    }
}

// --- Span Manipulation APIs ---

/// Begins a new span, making it the current active span for the calling thread.
///
/// A span represents a unit of work or a specific period during the execution of a program.
/// Spans can be nested to represent parent-child relationships between operations.
///
/// # Arguments
///
/// * `name`: A human-readable name for the span (e.g., "database_query", "process_request").
/// * `kind`: An optional string categorizing the span (e.g., "client", "server", "producer", "consumer").
///           This can be used by tracing systems for semantic interpretation.
/// * `code_path`: An optional string representing the code location where the span is initiated
///                (e.g., "my_module::my_function").
///
/// # Returns
///
/// On success, returns a `Result` containing a tuple with the new `span::SpanId` and `span::TraceId`.
/// The `TraceId` is shared by all spans belonging to the same trace (i.e., originating from the same root span).
/// The `SpanId` uniquely identifies this span within the trace.
///
/// On failure, returns a `TraceError`, typically `TraceError::LockPoisoned` if the underlying
/// tracer's lock was poisoned.
///
/// # Examples
///
/// ```
/// // Assuming trace::begin_span is in scope
/// // use crate::trace::begin_span;
/// // fn my_function() -> Result<(), trace::TraceError> {
/// //     let (span_id, trace_id) = begin_span(
/// //         "my_function_span",
/// //         Some("internal_operation"),
/// //         Some("my_app::my_module::my_function")
/// //     )?;
/// //     // ... do work ...
/// //     Ok(())
/// // }
/// ```
pub fn begin_span(
    name: &str,
    kind: Option<&str>,
    code_path: Option<&str>,
) -> Result<(span::SpanId, span::TraceId), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        Ok(tracer_guard.start_span(name, kind, code_path))
    })
}

/// Ends the current active span on the calling thread.
///
/// This marks the span as completed with a default status of `SpanStatus::Close` (successful).
/// If there was a parent span, it becomes the active span again.
///
/// # Returns
///
/// Returns `Ok(())` on successful completion.
/// Returns a `TraceError` (e.g., `TraceError::LockPoisoned`) if an error occurs,
/// such as failing to acquire a lock on the thread-local tracer.
///
/// # Panics
///
/// This function might implicitly cause a panic if called when no span is active,
/// depending on the underlying `LocalSpanManager::end_span` implementation if it expects a span to be present.
/// It's generally expected to be called after a corresponding `begin_span`.
pub fn end_span() -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.end_span(SpanStatus::Close); // Default to successful close
        Ok(())
    })
}

/// Ends the current active span on the calling thread with a specific status.
///
/// This allows for explicitly setting the outcome of the span (e.g., success, error).
/// If there was a parent span, it becomes the active span again.
///
/// # Arguments
///
/// * `status`: The `SpanStatus` to set for the ended span.
///
/// # Returns
///
/// Returns `Ok(())` on successful completion.
/// Returns a `TraceError` (e.g., `TraceError::LockPoisoned`) if an error occurs.
pub fn end_span_with_status(status: SpanStatus) -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.end_span(status);
        Ok(())
    })
}

/// Adds an attribute (a key-value pair) to the current active span on the calling thread.
///
/// Attributes provide additional context and details about the span. If the span is already
/// ended, adding an attribute might have no effect or be an error, depending on the
/// underlying `LocalSpanManager` implementation.
///
/// # Arguments
///
/// * `key`: The attribute key (e.g., "http.method", "db.statement").
/// * `value`: The attribute value, which can be any type that implements `Into<Ele>`.
///            `Ele` is an enum representing various primitive telemetry data types.
///
/// # Returns
///
/// Returns `Ok(())` if the attribute was successfully added (or queued to be added).
/// Returns a `TraceError` (e.g., `TraceError::LockPoisoned`) if an error occurs.
pub fn add_attr<V: Into<Ele>>(key: &str, value: V) -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.add_attr(key, value);
        Ok(())
    })
}

/// Adds an event to the current active span on the calling thread.
///
/// Events are timestamped occurrences within a span, often with their own set of attributes.
/// They represent significant moments or messages during the span's lifetime.
///
/// # Arguments
///
/// * `name`: A human-readable name for the event (e.g., "cache_miss", "user_login_failed").
/// * `attributes`: An optional vector of `Attribute`s to associate with this event.
///
/// # Returns
///
/// Returns `Ok(())` if the event was successfully added.
/// Returns a `TraceError` (e.g., `TraceError::LockPoisoned`) if an error occurs.
pub fn add_event(name: &str, attributes: Option<Vec<Attribute>>) -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.add_event(name, attributes);
        Ok(())
    })
}

/// Retrieves a clone of the current active span on the calling thread, if one exists.
///
/// This function provides read-only access to the current span's data.
///
/// # Returns
///
/// * `Ok(Some(span::Span))`: If there is an active span, a clone of it is returned.
/// * `Ok(None)`: If there is no active span on the current thread.
/// * `Err(TraceError)`: If an error occurs (e.g., `TraceError::LockPoisoned`).
pub fn current_span() -> Result<Option<span::Span>, TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let tracer_guard = tracer.read()?;
        Ok(tracer_guard.current_span())
    })
}

/// Lists clones of all spans currently on the local thread's span stack.
///
/// This includes the current active span and all its active parent spans on this thread.
/// The spans are typically ordered from the outermost (oldest on the stack) to the
/// innermost (most recently started/current active span).
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<span::Span>` with clones of the active spans,
/// or a `TraceError` if an error occurs. The vector will be empty if no spans are active.
pub fn list_spans() -> Result<Vec<span::Span>, TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let tracer_guard = tracer.read()?;
        Ok(tracer_guard.list_spans()) // This already returns Vec<Span>
    })
}

/// Retrieves span statistics for spans recorded by the tracer for the current thread.
///
/// This provides metrics and counts for spans, such as how many were started, ended,
/// and their respective statuses. This can be useful for analyzing the behavior of
/// traced operations and for debugging purposes.
///
/// # Returns
///
/// Returns a `Result` containing a `HashMap` where:
/// * Each key is a tuple consisting of:
///   - An optional string (e.g., for the span's kind)
///   - A string representing the span's name
///   - A `SpanStatus` indicating the final status of the span
/// * The corresponding value is a `span::SpanStats` struct containing statistical data
///   about the span (e.g., count, total duration).
///
/// Returns a `TraceError` if an error occurs (e.g., `TraceError::LockPoisoned`).
pub fn get_span_statistics(
) -> Result<HashMap<(Option<String>, String, span::SpanStatus), span::SpanStats>, TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let tracer_guard = tracer.read()?;
        Ok(tracer_guard.get_statistics())
    })
}

/// Retrieves all spans recorded by the tracer for the current thread.
///
/// This includes both active and inactive spans that have been started on the current thread.
/// The spans are typically returned in the order they were recorded.
///
/// # Returns
///
/// Returns a `Result` containing a `Vec<span::Span>` with clones of all spans recorded
/// for the current thread, or a `TraceError` if an error occurs. The vector may be empty
/// if no spans have been recorded.
pub fn all_spans() -> Result<Vec<span::Span>, TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let tracer_guard = tracer.read()?;
        Ok(tracer_guard.all_spans()) // This already returns Vec<Span>
    })
}

// --- Global Trace Information APIs ---

/// Retrieves clones of all spans from all registered threads known to the global tracer.
///
/// This function provides a snapshot of all active spans across the entire application,
/// grouped by their `ThreadId`. This can be useful for debugging or global state inspection.
///
/// Note: The exact set of spans returned depends on the state of each thread's
/// `LocalSpanManager` at the time of the call and their registration with the `GlobalSpanManager`.
///
/// # Returns
///
/// Returns a `Result` containing a `HashMap<ThreadId, Vec<span::Span>>`. Each key is a `ThreadId`,
/// and the value is a vector of `span::Span` clones representing the active spans for that thread.
/// Returns a `TraceError` if an error occurs (e.g., `TraceError::LockPoisoned` when accessing
/// the global tracer or individual local tracers).
pub fn all_thread_spans() -> Result<HashMap<ThreadId, Vec<span::Span>>, TraceError> {
    GLOBAL_TRACER.all_thread_spans() // No longer needs map_err as GLOBAL_TRACER methods now return Result<_, TraceError>
}

/// Retrieves clones of all active spans for a specific thread, identified by `thread_id`.
///
/// # Arguments
///
/// * `thread_id`: The `ThreadId` of the thread whose spans are to be retrieved.
///
/// # Returns
///
/// * `Ok(Some(Vec<span::Span>))`: If the specified thread is found and has active spans.
/// * `Ok(None)`: If the specified thread is not found or has no active spans.
/// * `Err(TraceError)`: If an error occurs (e.g., `TraceError::LockPoisoned`).
pub fn thread_spans(thread_id: ThreadId) -> Result<Option<Vec<span::Span>>, TraceError> {
    GLOBAL_TRACER.thread_spans(thread_id) // No longer needs map_err
}
