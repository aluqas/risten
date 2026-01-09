//! # Primitive Kernel Layer (Hook)
//!
//! The lowest-level entry point for event processing in Risten.
//!
//! Hooks are analogous to JavaScript event handlers: simple, universal, and
//! the foundation upon which all higher abstractions are built. Every Listener,
//! Router, and Handler ultimately becomes a Hook when executed.
//!
//! # Design Philosophy
//!
//! - **Atomic**: Hooks are the indivisible unit of event processing
//! - **Universal**: All framework abstractions compile down to Hooks
//! - **Low-Level Access**: Ecosystem plugins can target this layer directly,
//!   operating independently of higher-level conveniences
//!
//! # Use Cases
//!
//! - Observing events (logging, metrics, tracing)
//! - Filtering events before they reach listeners
//! - Building custom middleware without framework dependencies
//! - Wrapping Listener + Handler pipelines for execution

use crate::message::Message;
use std::{future::Future, pin::Pin};

/// Result of hook execution indicating whether to continue or stop propagation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    /// The event was observed or partially handled; continue to the next hook.
    Next,
    /// The event was fully handled; stop propagation to subsequent hooks.
    Stop,
}

/// The primitive kernel for event processing.
///
/// Hooks are the fundamental building blocks of Risten. Like JavaScript event
/// handlers, they receive an event and decide whether to continue propagation
/// (`Next`) or stop it (`Stop`).
///
/// # Layer Position
///
/// This is **Layer 1 (Primitive Kernel)** in the Risten architecture.
/// All higher abstractions ([`Listener`], [`Router`], [`Handler`]) are
/// converted to Hooks for actual execution.
///
/// # For Ecosystem Developers
///
/// If you're building plugins, middleware, or extensions, implementing `Hook`
/// directly gives you maximum control without depending on higher-level APIs.
///
/// # Static vs Dynamic Dispatch
///
/// This trait uses native `async fn` for zero-cost static dispatch.
/// For dynamic dispatch (e.g., in registries or collections), use [`DynHook`].
///
/// [`Listener`]: crate::Listener
/// [`Router`]: crate::Router
/// [`Handler`]: crate::Handler
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
