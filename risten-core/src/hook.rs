//! Hook trait for event processing.

use crate::message::Message;
use std::{future::Future, pin::Pin};

/// Result of hook execution indicating whether to continue or stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    /// The event was handled or observed, continue to the next hook.
    Next,
    /// Stop propagation of the event to subsequent hooks.
    Stop,
}

/// A low-level primitive for injecting logic into the event processing pipeline.
///
/// Hooks are the fundamental building blocks of the Risten system.
/// They can be used for:
/// - Observing events (logging, metrics)
/// - Filtering events
/// - Executing pipelines (wrapping Listener + Handler)
///
/// This trait uses native `async fn` for zero-cost static dispatch.
/// For dynamic dispatch (e.g. in Registry), use [`DynHook`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Hook<{E}>`",
    label = "missing `Hook` implementation",
    note = "Hooks must implement `on_event` for the specific event type `{E}`."
)]
pub trait Hook<E: Message>: Send + Sync + 'static {
    /// Called when an event is dispatched.
    fn on_event(
        &self,
        event: &E,
    ) -> impl Future<Output = Result<HookResult, Box<dyn std::error::Error + Send + Sync>>> + Send;
}

/// Dynamic object-safe version of [`Hook`].
///
/// Use this trait when you need runtime polymorphism (e.g., in a Registry).
pub trait DynHook<E: Message>: Send + Sync + 'static {
    /// Called when an event is dispatched (dynamic dispatch version).
    fn on_event_dyn<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<HookResult, Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + 'a,
        >,
    >;
}

// Blanket implementation: Any type implementing Hook implements DynHook automatically.
impl<E: Message, T: Hook<E>> DynHook<E> for T {
    fn on_event_dyn<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<HookResult, Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(self.on_event(event))
    }
}

// Allow Box<dyn DynHook> to be used where Hook is expected.
impl<E: Message> Hook<E> for Box<dyn DynHook<E>> {
    async fn on_event(
        &self,
        event: &E,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        self.on_event_dyn(event).await
    }
}
