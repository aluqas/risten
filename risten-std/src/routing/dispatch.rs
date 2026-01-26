//! # Dispatch-based Router using Inventory Collection
//!
//! This module provides a router that automatically collects handlers
//! registered via the `inventory` crate and executes them in parallel.
//!
//! # Overview
//!
//! The `DispatchRouter` is the primary implementation for inventory-based
//! handler collection. Handlers are registered globally using the `#[subscribe]`
//! macro or manually via `inventory::submit!`.
//!
//! # Example
//!
//! ```rust,ignore
//! use risten::{subscribe, DispatchRouter};
//!
//! // Register a handler using the macro
//! #[subscribe]
//! async fn on_message(event: MessageEvent) {
//!     println!("Received: {:?}", event);
//! }
//!
//! // Create and use the router
//! let router = DispatchRouter::<MessageEvent>::new();
//! router.route(&event).await?;
//! ```

use futures::future::join_all;
use risten_core::{DynHandler, ExtractError, Message, RouteResult, Router};
use std::any::{Any, TypeId};
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

/// Type-erased handler trait for dynamic dispatch.
///
/// This trait allows handlers of different concrete types to be stored
/// in a single collection and called uniformly.
pub trait ErasedHandler: Send + Sync {
    /// Execute the handler with a type-erased event.
    ///
    /// The event is passed as `&dyn Any` and downcast to the concrete type internally.
    fn call_erased<'a>(
        &'a self,
        event: &'a (dyn Any + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), ExtractError>> + Send + 'a>>;
}

/// Wrapper to implement [`ErasedHandler`] for a typed handler.
///
/// This struct bridges the gap between strongly-typed handlers and
/// the type-erased dispatch system.
pub struct ErasedHandlerWrapper<E, H> {
    /// The wrapped handler.
    handler: H,
    _phantom: std::marker::PhantomData<E>,
}

impl<E, H> ErasedHandlerWrapper<E, H> {
    /// Create a new wrapper around a typed handler.
    pub const fn new(handler: H) -> Self {
        Self {
            handler,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E, H> ErasedHandler for ErasedHandlerWrapper<E, H>
where
    E: Message + Clone + 'static,
    H: DynHandler<E, Output = Result<(), ExtractError>> + Send + Sync,
{
    fn call_erased<'a>(
        &'a self,
        event: &'a (dyn Any + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<(), ExtractError>> + Send + 'a>> {
        let event_ref = event
            .downcast_ref::<E>()
            .expect("Type mismatch in ErasedHandler");
        let event_owned = event_ref.clone();
        self.handler.call_dyn(event_owned)
    }
}

/// Registration entry for a handler in the global registry.
///
/// This struct is submitted to `inventory` for automatic collection.
pub struct HandlerRegistration {
    /// The TypeId of the event this handler processes.
    pub type_id: TypeId,
    /// The type-erased handler.
    pub handler: &'static (dyn ErasedHandler + Send + Sync),
    /// Priority for execution ordering (higher = earlier).
    pub priority: i32,
}

inventory::collect!(HandlerRegistration);

/// Errors that can occur during dispatch routing.
#[derive(Debug, Error)]
pub enum DispatchError {
    /// An error occurred during argument extraction.
    #[error(transparent)]
    Extract(#[from] ExtractError),

    /// A generic error from handler execution.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A router that collects and executes handlers registered via `inventory`.
///
/// This router automatically discovers all handlers registered for event type `E`
/// and executes them in parallel when `route()` is called.
///
/// # Features
///
/// - **Automatic Collection**: No manual registration needed; handlers are
///   discovered at runtime from the global registry.
/// - **Parallel Execution**: All matching handlers run concurrently via `join_all`.
/// - **Priority Support**: Handlers can specify priority for ordering (future enhancement).
///
/// # Example
///
/// ```rust,ignore
/// let router = DispatchRouter::<MyEvent>::new();
///
/// // This will execute all handlers registered for MyEvent in parallel
/// let result = router.route(&my_event).await?;
/// println!("Executed {} handlers", result.executed_count);
/// ```
pub struct DispatchRouter<E> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E> DispatchRouter<E> {
    /// Create a new dispatch router for events of type `E`.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the number of handlers registered for event type `E`.
    pub fn handler_count() -> usize
    where
        E: 'static,
    {
        let target_type = TypeId::of::<E>();
        inventory::iter::<HandlerRegistration>()
            .filter(|reg| reg.type_id == target_type)
            .count()
    }
}

impl<E> Default for DispatchRouter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Router<E> for DispatchRouter<E>
where
    E: Message + Clone + 'static,
{
    type Error = DispatchError;

    async fn route(&self, event: &E) -> Result<RouteResult, Self::Error> {
        let target_type = TypeId::of::<E>();
        let any_event = event as &(dyn Any + Send + Sync);

        // Collect all handlers for this event type
        let handlers: Vec<_> = inventory::iter::<HandlerRegistration>()
            .filter(|reg| reg.type_id == target_type)
            .collect();

        if handlers.is_empty() {
            return Ok(RouteResult::continued());
        }

        let handler_count = handlers.len();

        // Execute all handlers in parallel
        let futures: Vec<_> = handlers
            .iter()
            .map(|reg| reg.handler.call_erased(any_event))
            .collect();

        let results = join_all(futures).await;

        // Check for errors
        for res in results {
            if let Err(e) = res {
                return Err(DispatchError::Extract(e));
            }
        }

        Ok(RouteResult::with_count(handler_count))
    }
}

/// A router that executes handlers sequentially instead of in parallel.
///
/// Use this when handler order matters or when you need to stop
/// processing on the first error.
///
/// # Example
///
/// ```rust,ignore
/// let router = SequentialDispatchRouter::<MyEvent>::new();
/// router.route(&event).await?;
/// ```
pub struct SequentialDispatchRouter<E> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E> SequentialDispatchRouter<E> {
    /// Create a new sequential dispatch router for events of type `E`.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> Default for SequentialDispatchRouter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Router<E> for SequentialDispatchRouter<E>
where
    E: Message + Clone + 'static,
{
    type Error = DispatchError;

    async fn route(&self, event: &E) -> Result<RouteResult, Self::Error> {
        let target_type = TypeId::of::<E>();
        let any_event = event as &(dyn Any + Send + Sync);

        // Collect all handlers for this event type, sorted by priority
        let mut handlers: Vec<_> = inventory::iter::<HandlerRegistration>()
            .filter(|reg| reg.type_id == target_type)
            .collect();

        // Sort by priority (higher priority = earlier execution)
        handlers.sort_by(|a, b| b.priority.cmp(&a.priority));

        if handlers.is_empty() {
            return Ok(RouteResult::continued());
        }

        let mut executed_count = 0;

        // Execute handlers sequentially
        for reg in handlers {
            reg.handler.call_erased(any_event).await?;
            executed_count += 1;
        }

        Ok(RouteResult::with_count(executed_count))
    }
}

/// Execution mode for dispatch routers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DispatchMode {
    /// Execute all handlers in parallel (default).
    #[default]
    Parallel,
    /// Execute handlers sequentially, respecting priority order.
    Sequential,
}

/// A configurable dispatch router that supports both parallel and sequential execution.
///
/// This router allows you to choose the execution mode at construction time.
///
/// # Example
///
/// ```rust,ignore
/// // Parallel execution (default)
/// let router = ConfigurableDispatchRouter::<MyEvent>::new();
///
/// // Sequential execution
/// let router = ConfigurableDispatchRouter::<MyEvent>::sequential();
/// ```
pub struct ConfigurableDispatchRouter<E> {
    mode: DispatchMode,
    _phantom: std::marker::PhantomData<E>,
}

impl<E> ConfigurableDispatchRouter<E> {
    /// Create a new router with parallel execution (default).
    pub fn new() -> Self {
        Self {
            mode: DispatchMode::Parallel,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new router with sequential execution.
    pub fn sequential() -> Self {
        Self {
            mode: DispatchMode::Sequential,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new router with the specified execution mode.
    pub fn with_mode(mode: DispatchMode) -> Self {
        Self {
            mode,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the current execution mode.
    pub fn mode(&self) -> DispatchMode {
        self.mode
    }
}

impl<E> Default for ConfigurableDispatchRouter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Router<E> for ConfigurableDispatchRouter<E>
where
    E: Message + Clone + 'static,
{
    type Error = DispatchError;

    async fn route(&self, event: &E) -> Result<RouteResult, Self::Error> {
        match self.mode {
            DispatchMode::Parallel => {
                let router = DispatchRouter::<E>::new();
                router.route(event).await
            }
            DispatchMode::Sequential => {
                let router = SequentialDispatchRouter::<E>::new();
                router.route(event).await
            }
        }
    }
}
