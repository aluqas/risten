//! # Router: Event Distribution Engine
//!
//! A Router is the core abstraction for **"how events flow"** in Risten.
//! It encapsulates three key responsibilities:
//!
//! 1. **Condition Dispatch (Static Match)**: Route events based on type or value
//!    with zero-cost, compile-time optimization.
//! 2. **Collection**: Aggregate handlers defined across the codebase (e.g., via `inventory`).
//! 3. **Execution**: Control how collected/selected handlers run (sequential, parallel, all-at-once).
//!
//! # Design Philosophy
//!
//! - **Router knows nothing about Extractors**: A Router simply calls `Handler::call(event)`.
//!   How the Handler resolves its arguments (via Extractors) is the Handler's internal concern.
//! - **Zero-Copy by Default**: Routers take `&E` references, allowing multiple handlers
//!   to inspect the same event without cloning.
//! - **Composable**: Routers can be nested. Via [`RouterHook`], a Router becomes a [`Hook`],
//!   enabling hierarchical composition.
//!
//! # Standard Implementations
//!
//! `risten-std` provides concrete Router implementations:
//!
//! - **`StaticRouter`**: HList-based sequential execution (zero-cost, fully inlined).
//! - **`StaticFanoutRouter`**: HList-based parallel execution.
//! - **`DispatchRouter`**: Inventory-based collection with parallel execution.
//!
//! # Example
//!
//! ```rust,ignore
//! // Using StaticRouter for zero-cost sequential dispatch
//! let router = StaticRouter::new(static_hooks![
//!     LoggingHook,
//!     MetricsHook,
//!     my_handler_pipeline,
//! ]);
//!
//! // Using DispatchRouter for inventory-based collection
//! let router = DispatchRouter::<MyEvent>::new();
//! router.route(&event).await?;
//! ```

use crate::{
    error::BoxError,
    hook::{Hook, HookResult},
    message::Message,
};
use std::{future::Future, pin::Pin};

/// The result of a routing operation.
///
/// Indicates whether any handler in the router requested to stop propagation,
/// and optionally how many handlers were executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RouteResult {
    /// Whether any handler returned `Stop` during routing.
    pub stopped: bool,
    /// Number of handlers that were executed (optional tracking).
    pub executed_count: usize,
}

impl RouteResult {
    /// Create a result indicating no stop occurred and no handlers executed.
    pub const fn continued() -> Self {
        Self {
            stopped: false,
            executed_count: 0,
        }
    }

    /// Create a result indicating a stop occurred.
    pub const fn stopped() -> Self {
        Self {
            stopped: true,
            executed_count: 1,
        }
    }

    /// Create a result with a specific execution count.
    pub const fn with_count(count: usize) -> Self {
        Self {
            stopped: false,
            executed_count: count,
        }
    }

    /// Merge two results (useful for parallel execution).
    pub const fn merge(self, other: Self) -> Self {
        Self {
            stopped: self.stopped || other.stopped,
            executed_count: self.executed_count + other.executed_count,
        }
    }
}

/// The core event distribution engine.
///
/// A Router accepts an event reference and routes it to its internal collection
/// of handlers. The Router is responsible for:
///
/// - **Selection**: Determining which handlers should process this event.
/// - **Execution**: Running the selected handlers (sequentially, in parallel, etc.).
/// - **Aggregation**: Combining results from multiple handlers.
///
/// # Router vs Handler
///
/// **Important**: A Router does NOT know about Extractors. It simply calls
/// `handler.call(event)`. How the handler resolves its arguments is the
/// handler's internal concern (via `ExtractHandler` or similar wrappers).
///
/// # Execution Strategies
///
/// Different Router implementations provide different execution strategies:
///
/// - **Sequential**: Execute handlers one by one, stop on first `Stop` signal.
/// - **Parallel**: Execute all handlers concurrently using `join!`.
/// - **Conditional**: Match event patterns and route to specific handlers.
///
/// # Zero-Copy Design
///
/// The `route` method takes `&E` (a reference) rather than owned `E`.
/// This enables zero-copy routing where the same event can be processed
/// by multiple handlers without cloning.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot route events of type `{E}`",
    label = "missing `Router` implementation",
    note = "Implement `Router<{E}>` to handle event routing."
)]
pub trait Router<E: Message>: Send + Sync {
    /// The error type returned by routing operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Route the event through the registered handlers.
    ///
    /// Takes a reference to the event for zero-copy routing.
    /// Returns [`RouteResult`] indicating execution outcome.
    fn route(&self, event: &E) -> impl Future<Output = Result<RouteResult, Self::Error>> + Send;
}

/// Object-safe version of [`Router`] for dynamic dispatch.
///
/// Use this trait when you need runtime polymorphism (e.g., storing
/// heterogeneous routers in collections).
pub trait DynRouter<E>: Send + Sync {
    /// The error type returned by routing operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Route the event through the registered handlers (dynamic dispatch version).
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

/// Execution strategy marker for routers.
///
/// This is used by router implementations to indicate their execution behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Execute handlers sequentially, stop on first `Stop` signal.
    Sequential,
    /// Execute all handlers in parallel using async join.
    Parallel,
    /// Execute handlers based on pattern matching (only matching handlers run).
    Conditional,
}

/// A wrapper that allows a [`Router`] to be used as a [`Hook`].
///
/// This enables hierarchical composition of routers - a router can be
/// registered as a hook in another router, creating nested routing structures.
///
/// # Transparency Modes
///
/// By default, `RouterHook` is **transparent**: it always returns `Next` regardless
/// of whether internal handlers stopped propagation. This aligns with the Router's
/// role as a pass-through routing abstraction.
///
/// Use [`propagate_stop`](Self::propagate_stop) to create a non-transparent wrapper
/// that forwards the internal `Stop` signal to the parent hook chain.
///
/// # Example
///
/// ```rust,ignore
/// // Create a sub-router for message events
/// let message_router = StaticRouter::new(static_hooks![...]);
///
/// // Transparent (default): always returns Next
/// let transparent = RouterHook::new(message_router);
///
/// // Non-transparent: forwards Stop if any internal handler stopped
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

    /// Configure this hook to propagate `Stop` signals from internal handlers.
    ///
    /// When enabled, if any handler inside the router returns `Stop`,
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
