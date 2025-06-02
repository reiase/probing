mod span;

use crate::trace::span::{Attribute, Ele, SpanStatus, GLOBAL_TRACER, LOCAL_TRACER};
use std::collections::HashMap;
use std::sync::PoisonError;
use std::thread::ThreadId;

// --- Custom Error Type ---
#[derive(Debug)]
pub enum TraceError {
    LockPoisoned,
    // Add other error variants as needed
}

// Implement From<PoisonError> for TraceError to allow '?' to work with RwLock errors
impl<T> From<PoisonError<T>> for TraceError {
    fn from(_: PoisonError<T>) -> Self {
        TraceError::LockPoisoned
    }
}

// --- 操作Span的API ---

/// Begins a new span with the given name, kind, code_path, and initial attributes.
/// Returns the new SpanId and TraceId, or a TraceError if the tracer lock is poisoned.
pub fn begin_span(
    name: &str,
    kind: Option<&str>,
    code_path: Option<&str>,
    initial_attributes: Option<Vec<Attribute>>,
) -> Result<(span::SpanId, span::TraceId), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        Ok(tracer_guard.start_span(name, kind, code_path, initial_attributes))
    })
}

/// Ends the current active span.
/// By default, marks the span as successfully closed.
/// Returns a TraceError if the tracer lock is poisoned.
pub fn end_span() -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.end_span(SpanStatus::Close); // Default to successful close
        Ok(())
    })
}

/// Ends the current active span with a specific status.
/// Returns a TraceError if the tracer lock is poisoned.
pub fn end_span_with_status(status: SpanStatus) -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.end_span(status);
        Ok(())
    })
}

/// Adds an attribute (key-value pair) to the current active span.
/// Returns a TraceError if the tracer lock is poisoned.
pub fn add_attr<V: Into<Ele>>(key: &str, value: V) -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.add_attr(key, value);
        Ok(())
    })
}

/// Adds an event to the current active span, with optional attributes.
/// Returns a TraceError if the tracer lock is poisoned.
pub fn add_event(name: &str, attributes: Option<Vec<Attribute>>) -> Result<(), TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let mut tracer_guard = tracer.write()?;
        tracer_guard.add_event(name, attributes);
        Ok(())
    })
}

/// Gets a clone of the current active span, if any.
/// Returns a TraceError if the tracer lock is poisoned.
pub fn current_span() -> Result<Option<span::Span>, TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let tracer_guard = tracer.read()?;
        Ok(tracer_guard.current_span())
    })
}

/// Lists clones of all spans currently on the local thread's span stack (active spans and their parents).
/// The most recently started span is last in the vector.
/// Returns a TraceError if the tracer lock is poisoned.
pub fn list_spans() -> Result<Vec<span::Span>, TraceError> {
    LOCAL_TRACER.with(|tracer| {
        let tracer_guard = tracer.read()?;
        Ok(tracer_guard.list_spans()) // This already returns Vec<Span>
    })
}

// --- 获取全局信息的API ---

/// Retrieves clones of all spans from all registered threads.
pub fn all_thread_spans() -> HashMap<ThreadId, Vec<span::Span>> {
    GLOBAL_TRACER.all_thread_spans()
}

/// Retrieves clones of all spans for a specific thread.
pub fn thread_spans(thread_id: ThreadId) -> Option<Vec<span::Span>> {
    GLOBAL_TRACER.thread_spans(thread_id)
}
