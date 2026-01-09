//! Router core traits.
//!
//! A Router is the execution engine that processes events through its registered hooks.
//! It represents a collection of Hooks/Listeners and executes them in sequence or parallel.
//!
//! # Zero-Copy Routing
//!
//! Routers take event references (`&E`) instead of owned events, enabling zero-copy
//! event propagation through the routing pipeline. This avoids unnecessary clones
//! when routing events through multiple hooks.
//!
//! # Hierarchical Composition
//!
//! Routers can be composed hierarchically using [`RouterHook`], which wraps a Router
//! and implements [`Hook`] for it. This allows a router to be registered as a hook
//! in another router.
//!
//! ```rust,ignore
//! let sub_router = StaticRouter::new(static_hooks![...]);
//! let main_router = StaticRouter::new(static_hooks![
//!     RouterHook::new(sub_router),  // Nested router as a hook
//!     my_other_hook,
//! ]);
//! ```

use crate::{
    error::BoxError,
    hook::{Hook, HookResult},
    message::Message,
};
use std::{future::Future, pin::Pin};

/// A router that executes hooks for an event.
///
/// Routers are the runtime execution engines in Risten. They hold a collection
/// of hooks (which may include Listeners, other Routers, etc.) and execute them
/// when an event is received.
///
/// # Zero-Copy Design
///
/// The `route` method takes a reference to the event (`&E`) rather than an owned
/// event. This enables zero-copy routing where the same event can be processed
/// by multiple hooks without cloning.
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
    fn route(&self, event: &E) -> impl Future<Output = Result<(), Self::Error>> + Send;
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
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
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
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        E: Message + 'a,
    {
        Box::pin(self.route(event))
    }
}

// ============================================================================
// Router as Hook (Hierarchical Composition)
// ============================================================================

/// A wrapper that allows a [`Router`] to be used as a [`Hook`].
///
/// This enables hierarchical composition of routers - a router can be
/// registered as a hook in another router, creating nested routing structures.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{RouterHook, StaticRouter, static_hooks};
///
/// // Create a sub-router for message events
/// let message_router = StaticRouter::new(static_hooks![...]);
///
/// // Use it as a hook in the main router
/// let main_router = StaticRouter::new(static_hooks![
///     RouterHook::new(message_router),
///     logging_hook,
/// ]);
/// ```
pub struct RouterHook<R> {
    router: R,
}

impl<R> RouterHook<R> {
    /// Create a new RouterHook wrapping the given router.
    pub fn new(router: R) -> Self {
        Self { router }
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
        // Route the event through the inner router (zero-copy, no clone needed)
        self.router
            .route(event)
            .await
            .map_err(|e| Box::new(e) as BoxError)?;
        Ok(HookResult::Next)
    }
}
