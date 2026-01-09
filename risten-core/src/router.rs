//! # Routing Layer (Router)
//!
//! An abstraction over [`Listener`]s that routes events to appropriate handlers.
//! From the outside, a Router is just another processing step; internally, it
//! manages complex dispatch decisions.
//!
//! # Layer Position
//!
//! This is **Layer 3 (Routing)** in the Risten architecture.
//! Routers bundle multiple Listeners or Hooks into a single composable unit.
//!
//! # Design Philosophy
//!
//! - **Abstraction**: Combines multiple Listeners/Hooks into one logical unit
//! - **Transparent**: Acts as a pass-through; callers don't see internal routing
//! - **Composable**: Via [`RouterHook`], a Router becomes a [`Hook`] itself,
//!   enabling infinite hierarchical composition
//!
//! # Zero-Copy Routing
//!
//! Routers take event references (`&E`) instead of owned events, enabling zero-copy
//! event propagation. The same event can be processed by multiple hooks without cloning.
//!
//! # Hierarchical Composition
//!
//! Routers can be nested using [`RouterHook`]:
//!
//! ```rust,ignore
//! let sub_router = StaticRouter::new(static_hooks![...]);
//! let main_router = StaticRouter::new(static_hooks![
//!     RouterHook::new(sub_router),  // Router as a Hook
//!     logging_hook,
//! ]);
//! ```
//!
//! [`Listener`]: crate::Listener
//! [`Hook`]: crate::Hook

use crate::{
    error::BoxError,
    hook::{Hook, HookResult},
    message::Message,
};
use std::{future::Future, pin::Pin};

/// The result of a routing operation.
///
/// Indicates whether any hook in the router requested to stop propagation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RouteResult {
    /// Whether any hook returned `Stop` during routing.
    pub stopped: bool,
}

impl RouteResult {
    /// Create a new RouteResult indicating no stop occurred.
    pub const fn continued() -> Self {
        Self { stopped: false }
    }

    /// Create a new RouteResult indicating a stop occurred.
    pub const fn stopped() -> Self {
        Self { stopped: true }
    }
}

/// A router that executes hooks for an event.
///
/// Routers are the runtime execution engines in Risten. They hold a collection
/// of hooks (which may include Listeners, other Routers, etc.) and execute them
/// when an event is received.
///
/// # Layer Position
///
/// This is **Layer 3 (Routing)** in the Risten architecture.
/// Routers are abstractions over Listeners that handle event dispatch.
///
/// # Zero-Copy Design
///
/// The `route` method takes a reference to the event (`&E`) rather than an owned
/// event. This enables zero-copy routing where the same event can be processed
/// by multiple hooks without cloning.
///
/// # Transparency
///
/// Routers are "transparent" — they route events to internal handlers but don't
/// inherently consume events. The [`RouteResult`] optionally reports whether
/// any internal hook stopped propagation.
///
/// # Hierarchy
///
/// Routers can be composed hierarchically by implementing `Hook` for `Router`,
/// allowing a router to be registered as a hook in another router.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot route events of type `{E}`",
    label = "missing `Router` implementation",
    note = "Implement `Router<{E}>` to handle event routing."
)]
pub trait Router<E: Message>: Send + Sync {
    /// The error type returned by routing operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Route the event through the registered hooks.
    ///
    /// Takes a reference to the event for zero-copy routing.
    /// Returns [`RouteResult`] indicating whether any hook stopped propagation.
    fn route(&self, event: &E) -> impl Future<Output = Result<RouteResult, Self::Error>> + Send;
}

/// Object-safe version of [`Router`] for dynamic dispatch.
///
/// Use this trait when you need runtime polymorphism (e.g., storing routers in collections).
pub trait DynRouter<E>: Send + Sync {
    /// The error type returned by routing operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Route the event through the registered hooks (dynamic dispatch version).
    ///
    /// Takes a reference to the event for zero-copy routing.
    fn route<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<Box<dyn Future<Output = Result<RouteResult, Self::Error>> + Send + 'a>>
    where
        E: Message + 'a;
}

// Blanket implementation: Any type implementing Router implements DynRouter automatically.
impl<T, E> DynRouter<E> for T
where
    T: Router<E>,
    E: Message,
{
    type Error = T::Error;

    fn route<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<Box<dyn Future<Output = Result<RouteResult, Self::Error>> + Send + 'a>>
    where
        E: Message + 'a,
    {
        Box::pin(Router::route(self, event))
    }
}

/// A wrapper that allows a [`Router`] to be used as a [`Hook`].
///
/// This enables hierarchical composition of routers - a router can be
/// registered as a hook in another router, creating nested routing structures.
///
/// # Transparency Modes
///
/// By default, `RouterHook` is **transparent**: it always returns `Next` regardless
/// of whether internal hooks stopped propagation. This aligns with the Router's
/// role as a pass-through routing abstraction.
///
/// Use [`propagate_stop`](Self::propagate_stop) to create a non-transparent wrapper
/// that forwards the internal `Stop` signal to the parent hook chain.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{RouterHook, StaticRouter, static_hooks};
///
/// // Create a sub-router for message events
/// let message_router = StaticRouter::new(static_hooks![...]);
///
/// // Transparent (default): always returns Next
/// let transparent = RouterHook::new(message_router);
///
/// // Non-transparent: forwards Stop if any internal hook stopped
/// let non_transparent = RouterHook::new(message_router).propagate_stop();
/// ```
pub struct RouterHook<R> {
    router: R,
    propagate_stop: bool,
}

impl<R> RouterHook<R> {
    /// Create a new transparent RouterHook wrapping the given router.
    ///
    /// By default, this hook always returns `Next` after routing,
    /// making the router transparent to the parent hook chain.
    pub fn new(router: R) -> Self {
        Self {
            router,
            propagate_stop: false,
        }
    }

    /// Configure this hook to propagate `Stop` signals from internal hooks.
    ///
    /// When enabled, if any hook inside the router returns `Stop`,
    /// this wrapper will also return `Stop` to the parent chain.
    pub fn propagate_stop(mut self) -> Self {
        self.propagate_stop = true;
        self
    }

    /// Get a reference to the inner router.
    pub fn inner(&self) -> &R {
        &self.router
    }

    /// Consume this wrapper and return the inner router.
    pub fn into_inner(self) -> R {
        self.router
    }
}

impl<E, R> Hook<E> for RouterHook<R>
where
    E: Message + Sync,
    R: Router<E> + 'static,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        let result = self
            .router
            .route(event)
            .await
            .map_err(|e| Box::new(e) as BoxError)?;

        if self.propagate_stop && result.stopped {
            Ok(HookResult::Stop)
        } else {
            Ok(HookResult::Next)
        }
    }
}
