//! Testing utilities for Risten.
//!
//! This module provides utilities to make testing hooks, listeners, and handlers easier.
//!
//! # Features
//!
//! - [`MockContext`]: A mock extractor for testing handlers without real dependencies
//! - [`RecordingHook`]: A hook that records all events it receives
//! - [`SpyListener`]: A listener that records events and can be controlled
//! - [`TestRouter`]: A simple test router with inspection capabilities

use risten_core::{BoxError, FromEvent, Handler, Hook, HookResult, Listener, Message};
use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

// ============================================================================
// Mock Context
// ============================================================================

/// A mock context for testing handlers that use extraction.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Clone)]
/// struct MyContext {
///     user_id: u64,
/// }
///
/// // In your test:
/// let ctx = MockContext::new(MyContext { user_id: 42 });
/// let extracted = ctx.extract::<MyContext>(&event);
/// ```
#[derive(Clone)]
pub struct MockContext<T> {
    value: T,
}

impl<T: Clone> MockContext<T> {
    /// Create a new mock context with the given value.
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Extract the mock value (for testing).
    pub fn extract(&self) -> T {
        self.value.clone()
    }
}

impl<E, T: Clone + Send + Sync + 'static> FromEvent<E> for MockContext<T> {
    type Error = Infallible;

    fn from_event(_event: &E) -> Result<Self, Self::Error> {
        // Note: In real usage, you'd need to provide the context via thread-local or similar
        unimplemented!("MockContext::from_event should not be called directly in tests")
    }
}

// ============================================================================
// Recording Hook
// ============================================================================

/// A hook that records all events it receives.
///
/// Useful for verifying that events are being routed correctly.
///
/// # Example
///
/// ```rust,ignore
/// let recorder = RecordingHook::<MyEvent>::new();
/// let recorder_clone = recorder.clone();
///
/// // Use in router...
/// router.route(&event).await;
///
/// // Check what was recorded
/// let events = recorder_clone.events();
/// assert_eq!(events.len(), 1);
/// ```
pub struct RecordingHook<E: Clone> {
    events: Arc<Mutex<Vec<E>>>,
    result: HookResult,
}

impl<E: Clone> RecordingHook<E> {
    /// Create a new recording hook that returns `Next`.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            result: HookResult::Next,
        }
    }

    /// Create a recording hook that returns a specific result.
    pub fn with_result(result: HookResult) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            result,
        }
    }

    /// Get a clone of the recorded events.
    pub fn events(&self) -> Vec<E> {
        self.events.lock().unwrap().clone()
    }

    /// Get the number of recorded events.
    pub fn count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Clear all recorded events.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl<E: Clone> Default for RecordingHook<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Clone> Clone for RecordingHook<E> {
    fn clone(&self) -> Self {
        Self {
            events: self.events.clone(),
            result: self.result,
        }
    }
}

impl<E: Message + Clone + Sync> Hook<E> for RecordingHook<E> {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(self.result)
    }
}

// ============================================================================
// Spy Listener
// ============================================================================

/// A listener that records events and can be programmed to return specific results.
///
/// # Example
///
/// ```rust,ignore
/// let spy = SpyListener::new();
/// spy.set_output(Some(processed_event));
///
/// let result = spy.listen(&input_event).await;
/// assert!(result.is_ok());
/// ```
pub struct SpyListener<In: Clone, Out: Clone> {
    inputs: Arc<Mutex<Vec<In>>>,
    output: Arc<Mutex<Option<Out>>>,
    should_error: Arc<Mutex<Option<String>>>,
}

impl<In: Clone, Out: Clone> SpyListener<In, Out> {
    /// Create a new spy listener.
    pub fn new() -> Self {
        Self {
            inputs: Arc::new(Mutex::new(Vec::new())),
            output: Arc::new(Mutex::new(None)),
            should_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the output to return.
    pub fn set_output(&self, output: Option<Out>) {
        *self.output.lock().unwrap() = output;
    }

    /// Set an error to return.
    pub fn set_error(&self, error: impl Into<String>) {
        *self.should_error.lock().unwrap() = Some(error.into());
    }

    /// Clear error state.
    pub fn clear_error(&self) {
        *self.should_error.lock().unwrap() = None;
    }

    /// Get recorded inputs.
    pub fn inputs(&self) -> Vec<In> {
        self.inputs.lock().unwrap().clone()
    }

    /// Get the number of times listen was called.
    pub fn call_count(&self) -> usize {
        self.inputs.lock().unwrap().len()
    }
}

impl<In: Clone, Out: Clone> Default for SpyListener<In, Out> {
    fn default() -> Self {
        Self::new()
    }
}

impl<In: Clone, Out: Clone> Clone for SpyListener<In, Out> {
    fn clone(&self) -> Self {
        Self {
            inputs: self.inputs.clone(),
            output: self.output.clone(),
            should_error: self.should_error.clone(),
        }
    }
}

impl<In, Out> Listener<In> for SpyListener<In, Out>
where
    In: Message + Clone + Sync,
    Out: Message + Clone,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        self.inputs.lock().unwrap().push(event.clone());

        if let Some(ref err) = *self.should_error.lock().unwrap() {
            return Err(err.clone().into());
        }

        Ok(self.output.lock().unwrap().clone())
    }
}

// ============================================================================
// Counting Handler
// ============================================================================

/// A handler that counts invocations.
///
/// # Example
///
/// ```rust,ignore
/// let counter = CountingHandler::new();
/// let counter_clone = counter.clone();
///
/// // Use in pipeline...
/// pipeline.call(event).await;
///
/// assert_eq!(counter_clone.count(), 1);
/// ```
pub struct CountingHandler {
    count: Arc<AtomicUsize>,
}

impl CountingHandler {
    /// Create a new counting handler.
    pub fn new() -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the current count.
    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Reset the counter.
    pub fn reset(&self) {
        self.count.store(0, Ordering::SeqCst);
    }
}

impl Default for CountingHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CountingHandler {
    fn clone(&self) -> Self {
        Self {
            count: self.count.clone(),
        }
    }
}

impl<E: Message> Handler<E> for CountingHandler {
    type Output = ();

    async fn call(&self, _input: E) -> Self::Output {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

// ============================================================================
// Pass-through Listener
// ============================================================================

/// A simple listener that passes events through unchanged.
///
/// Useful as a starting point in test pipelines.
pub struct PassthroughListener<E>(std::marker::PhantomData<E>);

impl<E> PassthroughListener<E> {
    /// Create a new passthrough listener.
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<E> Default for PassthroughListener<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Message + Clone + Sync> Listener<E> for PassthroughListener<E> {
    type Output = E;

    async fn listen(&self, event: &E) -> Result<Option<Self::Output>, BoxError> {
        Ok(Some(event.clone()))
    }
}
