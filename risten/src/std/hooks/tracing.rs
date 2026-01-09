use crate::{
    core::{error::BoxError, message::Message},
    flow::hook::{Hook, HookResult},
};
use std::fmt::Debug;

/// Trait for events that support distributed tracing.
///
/// Implementing this trait allows the `TracingHook` wrapper to link new spans to
/// parent spans propagated via the event (e.g., from HTTP headers).
pub trait Traceable {
    /// Return the Trace ID if available (e.g. "4bf92f3577b34da6a3ce929d0e0e4736").
    fn trace_id(&self) -> Option<&str> {
        None
    }

    /// Return the Span ID of the parent span if available.
    fn span_id(&self) -> Option<&str> {
        None
    }
}

/// A Hook wrapper that instruments execution with a `tracing` Span.
///
/// This wrapper creates a span for the execution of the inner hook (or chain).
/// If the event implements `Traceable` and `tracing` feature is enabled,
/// it attempts to associate the span with the trace ID.
pub struct TracingHook<H> {
    inner: H,
    name: &'static str,
}

impl<H> TracingHook<H> {
    /// Create a new `TracingHook` wrapper around a hook.
    pub const fn new(inner: H, name: &'static str) -> Self {
        Self { inner, name }
    }
}

impl<H: Clone> Clone for TracingHook<H> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            name: self.name,
        }
    }
}

impl<H: Copy> Copy for TracingHook<H> {}

#[cfg(feature = "tracing")]
use tracing::Instrument;

impl<E, H> Hook<E> for TracingHook<H>
where
    E: Message + Debug + Sync + Traceable,
    H: Hook<E>,
{
    #[cfg(feature = "tracing")]
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        let span = if let Some(trace_id) = event.trace_id() {
            tracing::info_span!(
                "event_process",
                hook = %self.name,
                trace_id = %trace_id,
                span_id = %event.span_id().unwrap_or(""),
                event = ?event
            )
        } else {
            tracing::info_span!(
                "event_process",
                hook = %self.name,
                event = ?event
            )
        };

        // We use an async block to instrument the future returned by inner.on_event
        async move { self.inner.on_event(event).await }
            .instrument(span)
            .await
    }

    #[cfg(not(feature = "tracing"))]
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        // Fallback or no-op if tracing is disabled
        self.inner.on_event(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct MockEvent {
        trace_id: Option<String>,
        span_id: Option<String>,
    }

    // impl Message for MockEvent {} - Covered by blanket impl

    impl Traceable for MockEvent {
        fn trace_id(&self) -> Option<&str> {
            self.trace_id.as_deref()
        }
        fn span_id(&self) -> Option<&str> {
            self.span_id.as_deref()
        }
    }

    #[derive(Clone, Copy)]
    struct MockHook;

    impl Hook<MockEvent> for MockHook {
        async fn on_event(&self, _event: &MockEvent) -> Result<HookResult, BoxError> {
            Ok(HookResult::Next)
        }
    }

    #[tokio::test]
    async fn test_tracing_hook_passthrough() {
        let hook = TracingHook::new(MockHook, "test_hook");
        // Check clone
        let hook = hook.clone();

        let event = MockEvent {
            trace_id: Some("trace_123".to_string()),
            span_id: Some("span_456".to_string()),
        };

        let result = hook.on_event(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tracing_hook_no_trace_id() {
        let hook = TracingHook::new(MockHook, "test_hook");
        let event = MockEvent {
            trace_id: None,
            span_id: None,
        };

        let result = hook.on_event(&event).await;
        assert!(result.is_ok());
    }
}
