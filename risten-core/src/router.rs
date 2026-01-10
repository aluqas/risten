//! # Dispatch Engine Layer (Router)
//!
//! A Router is a component responsible for dispatching an event to one or more handlers.
//! Unlike a Listener (which focuses on event interpretation and domain logic),
//! a Router focuses on the **mechanics of distribution**.
//!
//! # Layer Position
//!
//! This is **Layer 3 (Routing)** in the Risten architecture.
//! It is typically used *inside* a Listener to dispatch the interpreted event to
//! registered handlers.
//!
//! # Roles
//!
//! - **Selection**: Find the right handler for the event (e.g., Match by command name).
//! - **Distribution**: Send the event to all interested parties (Fanout).
//! - **Chaining**: Execute handlers in a specific order (Middleware).
//!
//! # Zero-Copy
//!
//! Routers typically take a reference to the event (`&E`) to allow multiple
//! handlers to inspect the same event without cloning.

use crate::{
    error::BoxError,
    hook::{Hook, HookResult},
    message::Message,
};
use std::{future::Future, pin::Pin};

/// The result of a routing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RouteResult {
    /// Whether any hook returned `Stop` during routing.
    pub stopped: bool,
}

impl RouteResult {
    pub const fn continued() -> Self {
        Self { stopped: false }
    }
    pub const fn stopped() -> Self {
        Self { stopped: true }
    }
}

/// The dispatch engine interface.
///
/// A Router accepts an event and routes it to its internal collection of handlers.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot route events of type `{E}`",
    label = "missing `Router` implementation",
    note = "Implement `Router<{E}>` to handle event routing."
)]
pub trait Router<E: Message>: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn route(&self, event: &E) -> impl Future<Output = Result<RouteResult, Self::Error>> + Send;
}

pub trait DynRouter<E>: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn route<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<Box<dyn Future<Output = Result<RouteResult, Self::Error>> + Send + 'a>>
    where
        E: Message + 'a;
}

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
pub struct RouterHook<R> {
    router: R,
    propagate_stop: bool,
}

impl<R> RouterHook<R> {
    pub fn new(router: R) -> Self {
        Self {
            router,
            propagate_stop: false,
        }
    }

    pub fn propagate_stop(mut self) -> Self {
        self.propagate_stop = true;
        self
    }

    pub fn inner(&self) -> &R {
        &self.router
    }

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
