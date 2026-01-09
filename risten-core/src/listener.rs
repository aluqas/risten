//! # Rich Abstraction Layer (Listener)
//!
//! Wraps the primitive [`Hook`] layer to provide rich event processing features:
//! type transformation, filtering, and declarative pipeline composition.
//!
//! # Layer Position
//!
//! This is **Layer 2 (Rich Abstraction)** in the Risten architecture.
//! Listeners wrap Hook mechanics internally while exposing a higher-level API
//! focused on interpretation and decision-making rather than raw execution.
//!
//! # Design Philosophy
//!
//! - **Wrapper**: Internally uses Hook mechanisms, adding gatekeeping and transformation
//! - **Semantics**: "Listen and Decide" — not just "Do". Interpretation over action.
//! - **No Side Effects**: Listeners focus on policy (pass/block/transform).
//!   Side effects belong in [`Handler`].
//!
//! # Relationship to Other Layers
//!
//! | Layer | Role | Trait |
//! |-------|------|-------|
//! | Hook (L1) | Primitive execution | `on_event → Next/Stop` |
//! | **Listener (L2)** | **Interpretation & transformation** | **`listen → Option<Out>`** |
//! | Router (L3) | Event routing | `route → ()` |
//! | Handler (L4) | Terminal business logic | `call → Out` |
//!
//! # Declarative Pipeline Construction
//!
//! Listeners support method chaining for building pipelines:
//!
//! ```rust,ignore
//! let pipeline = AuthListener
//!     .filter(|e| !e.author.is_bot)           // Gatekeeping
//!     .then(|e| async move { e.load_ctx() })  // Async enrichment
//!     .map(|ctx| CommandContext::from(ctx))   // Transformation
//!     .handler(CommandHandler);               // → becomes a Hook
//! ```
//!
//! The final `.handler()` call produces a [`Pipeline`] which implements [`Hook`],
//! completing the cycle back to the primitive layer.
//!
//! [`Hook`]: crate::Hook
//! [`Handler`]: crate::Handler

use crate::{error::BoxError, handler::Handler, message::Message};
use std::{future::Future, pin::Pin};

/// A listener sits at the entry or intermediate points of the event pipeline.
///
/// Its role is to inspect an input event and optionally produce an output event
/// or decide to route it further. Listeners can perform async operations such as
/// database lookups for gatekeeping decisions.
///
/// # Gatekeeping vs Side Effects
///
/// Listeners should focus on **gatekeeping** (pass/block) and **transformation** (reshape).
/// Side effects (database writes, API calls) belong in [`Handler`].
///
/// # Example
///
/// ```rust,ignore
/// struct AuthListener;
///
/// impl Listener<RawEvent> for AuthListener {
///     type Output = AuthenticatedEvent;
///
///     async fn listen(&self, event: &RawEvent) -> Result<Option<Self::Output>, BoxError> {
///         // Async: Check user permissions from database
///         let user = db.get_user(event.user_id).await?;
///         if user.is_banned() {
///             return Ok(None); // Block the event
///         }
///         Ok(Some(AuthenticatedEvent { event: event.clone(), user }))
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Listener` for `{In}`",
    label = "missing `Listener` implementation",
    note = "Listeners must implement the `listen` method to process `{In}`."
)]
pub trait Listener<In: Message>: Send + Sync + 'static {
    /// The type of message this listener produces.
    type Output: Message;

    /// Inspects the input event and optionally transforms it into the Output type.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(output))`: The event was accepted and transformed.
    /// - `Ok(None)`: The event was rejected/filtered (no error, just skip).
    /// - `Err(e)`: An error occurred during processing.
    fn listen(
        &self,
        event: &In,
    ) -> impl Future<Output = Result<Option<Self::Output>, BoxError>> + Send;

    /// Chains this listener with another listener.
    ///
    /// The output of `self` becomes the input of `next`.
    fn and_then<Next>(self, next: Next) -> Chain<Self, Next>
    where
        Self: Sized,
        Next: Listener<Self::Output>,
    {
        Chain {
            first: self,
            second: next,
        }
    }

    /// Filters the output of this listener using a predicate.
    ///
    /// If the predicate returns `false`, the event is dropped (returns `None`).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let filtered = my_listener.filter(|event| event.is_important());
    /// ```
    fn filter<F>(self, predicate: F) -> Filter<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool + Send + Sync + 'static,
    {
        Filter {
            listener: self,
            predicate,
        }
    }

    /// Transforms the output of this listener using a synchronous mapper.
    ///
    /// The mapper always succeeds (the event passes through transformed).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mapped = my_listener.map(|event| ProcessedEvent::from(event));
    /// ```
    fn map<F, Out>(self, mapper: F) -> Map<Self, F>
    where
        Self: Sized,
        Out: Message,
        F: Fn(Self::Output) -> Out + Send + Sync + 'static,
    {
        Map {
            listener: self,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Transforms the output of this listener using an async mapper.
    ///
    /// Use this for transformations that require async operations (e.g., DB lookups).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let enriched = my_listener.then(|event| async move {
    ///     let user = db.get_user(event.user_id).await;
    ///     EnrichedEvent { event, user }
    /// });
    /// ```
    fn then<F, Out, Fut>(self, mapper: F) -> Then<Self, F>
    where
        Self: Sized,
        Out: Message,
        F: Fn(Self::Output) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Out> + Send,
    {
        Then {
            listener: self,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Filters and transforms the output in one step.
    ///
    /// If the mapper returns `None`, the event is dropped.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let filtered_mapped = my_listener.filter_map(|event| {
    ///     if event.is_valid() {
    ///         Some(ProcessedEvent::from(event))
    ///     } else {
    ///         None
    ///     }
    /// });
    /// ```
    fn filter_map<F, Out>(self, mapper: F) -> FilterMap<Self, F>
    where
        Self: Sized,
        Out: Message,
        F: Fn(Self::Output) -> Option<Out> + Send + Sync + 'static,
    {
        FilterMap {
            listener: self,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Connects this listener to a handler, creating a complete pipeline.
    ///
    /// The resulting `Pipeline` implements `Hook` and can be registered with a router.
    fn handler<H>(self, handler: H) -> Pipeline<Self, H>
    where
        Self: Sized,
        H: Handler<Self::Output>,
    {
        Pipeline {
            listener: self,
            handler,
        }
    }

    /// Catches errors from this listener and optionally recovers.
    ///
    /// The error handler receives the error and can return:
    /// - `Some(output)`: Recover with a fallback value
    /// - `None`: Swallow the error and filter out the event
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let resilient = my_listener.catch(|err| {
    ///     log::warn!("Listener error: {}", err);
    ///     None // Swallow error, filter event
    /// });
    /// ```
    fn catch<F>(self, handler: F) -> Catch<Self, F>
    where
        Self: Sized,
        F: Fn(BoxError) -> Option<Self::Output> + Send + Sync + 'static,
    {
        Catch::new(self, handler)
    }

    /// Boxes this listener for dynamic dispatch.
    ///
    /// This is useful when you need to store heterogeneous listeners
    /// or when the concrete type cannot be known at compile time.
    fn boxed(self) -> BoxListener<In, Self::Output>
    where
        Self: Sized,
        In: Sync,
    {
        BoxListener::new(self)
    }
}

/// A chain of two listeners.
///
/// Created by [`Listener::and_then`]. The first listener's output becomes the second's input.
pub struct Chain<A, B> {
    pub(crate) first: A,
    pub(crate) second: B,
}

impl<A, B, In> Listener<In> for Chain<A, B>
where
    In: Message + Sync,
    A: Listener<In>,
    A::Output: Sync,
    B: Listener<A::Output>,
{
    type Output = B::Output;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(intermediate) = self.first.listen(event).await? else {
            return Ok(None);
        };
        self.second.listen(&intermediate).await
    }
}

/// A listener that filters events based on a predicate.
///
/// Created by [`Listener::filter`].
pub struct Filter<L, F> {
    listener: L,
    predicate: F,
}

impl<L, F, In> Listener<In> for Filter<L, F>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Clone + Sync,
    F: Fn(&L::Output) -> bool + Send + Sync + 'static,
{
    type Output = L::Output;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        if (self.predicate)(&output) {
            Ok(Some(output))
        } else {
            Ok(None)
        }
    }
}

/// A listener that transforms events using a synchronous mapper.
///
/// Created by [`Listener::map`].
pub struct Map<L, F, Out = ()> {
    listener: L,
    mapper: F,
    _phantom: std::marker::PhantomData<Out>,
}

impl<L, F, In, Out> Listener<In> for Map<L, F, Out>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    Out: Message,
    F: Fn(L::Output) -> Out + Send + Sync + 'static,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        Ok(Some((self.mapper)(output)))
    }
}

/// A listener that transforms events using an async mapper.
///
/// Created by [`Listener::then`].
pub struct Then<L, F, Out = ()> {
    listener: L,
    mapper: F,
    _phantom: std::marker::PhantomData<Out>,
}

impl<L, F, In, Out, Fut> Listener<In> for Then<L, F, Out>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    Out: Message,
    F: Fn(L::Output) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Out> + Send,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        Ok(Some((self.mapper)(output).await))
    }
}

/// A listener that filters and transforms events in one step.
///
/// Created by [`Listener::filter_map`].
pub struct FilterMap<L, F, Out = ()> {
    listener: L,
    mapper: F,
    _phantom: std::marker::PhantomData<Out>,
}

impl<L, F, In, Out> Listener<In> for FilterMap<L, F, Out>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    Out: Message,
    F: Fn(L::Output) -> Option<Out> + Send + Sync + 'static,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        let Some(output) = self.listener.listen(event).await? else {
            return Ok(None);
        };
        Ok((self.mapper)(output))
    }
}

/// A complete pipeline connecting a Listener to a Handler.
///
/// Implements [`Hook`] so it can be registered with a router.
///
/// The pipeline executes in two phases:
/// 1. **Listener Phase** (async): Gatekeeping and transformation
/// 2. **Handler Phase** (async): Business logic and side effects
pub struct Pipeline<L, H> {
    /// The listener component (gatekeeper/transformer).
    pub listener: L,
    /// The handler component (action/endpoint).
    pub handler: H,
}

use crate::{
    handler::HandlerResult,
    hook::{Hook, HookResult},
    response::IntoResponse,
};

impl<L, H, In> Hook<In> for Pipeline<L, H>
where
    In: Message + Sync,
    L: Listener<In>,
    H: Handler<L::Output>,
    L::Output: Send + Sync,
    H::Output: HandlerResult + IntoResponse,
{
    async fn on_event(
        &self,
        event: &In,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        match self.listener.listen(event).await {
            Ok(Some(out)) => {
                let result = self.handler.call(out).await;
                result.into_response()
            }
            Ok(None) => Ok(HookResult::Next),
            Err(e) => Err(e),
        }
    }
}

/// A boxed, type-erased listener for dynamic dispatch.
///
/// Use this when you need to store heterogeneous listeners in collections
/// or when the concrete listener type cannot be known at compile time.
///
/// # Example
///
/// ```rust,ignore
/// let listeners: Vec<BoxListener<MyEvent, ProcessedEvent>> = vec![
///     BoxListener::new(AuthListener),
///     BoxListener::new(ValidationListener),
/// ];
/// ```
pub struct BoxListener<In, Out> {
    inner: Box<dyn DynListener<In, Output = Out>>,
}

impl<In, Out> BoxListener<In, Out>
where
    In: Message,
    Out: Message,
{
    /// Create a new boxed listener from any `Listener` implementation.
    pub fn new<L>(listener: L) -> Self
    where
        L: Listener<In, Output = Out>,
        In: Sync,
    {
        Self {
            inner: Box::new(listener),
        }
    }
}

impl<In, Out> Listener<In> for BoxListener<In, Out>
where
    In: Message + Sync,
    Out: Message,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        self.inner.listen_dyn(event).await
    }
}

/// Object-safe version of [`Listener`] for dynamic dispatch.
///
/// This trait is automatically implemented for any `Listener` and is used
/// internally by [`BoxListener`].
pub trait DynListener<In>: Send + Sync + 'static {
    /// The output type of this listener.
    type Output: Message;

    /// Object-safe listen method.
    fn listen_dyn<'a>(
        &'a self,
        event: &'a In,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self::Output>, BoxError>> + Send + 'a>>;
}

impl<L, In> DynListener<In> for L
where
    L: Listener<In>,
    In: Message + Sync,
{
    type Output = L::Output;

    fn listen_dyn<'a>(
        &'a self,
        event: &'a In,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self::Output>, BoxError>> + Send + 'a>> {
        Box::pin(self.listen(event))
    }
}

/// A listener that catches errors from the inner listener and optionally recovers.
///
/// The error handler can return `Some(output)` to recover with a fallback value,
/// or `None` to swallow the error and filter out the event.
///
/// # Example
///
/// ```rust,ignore
/// let resilient = my_listener.catch(|err| {
///     log::warn!("Listener error: {}", err);
///     None // Swallow error, filter event
/// });
/// ```
pub struct Catch<L, F> {
    listener: L,
    handler: F,
}

impl<L, F> Catch<L, F> {
    /// Create a new catch listener.
    pub fn new(listener: L, handler: F) -> Self {
        Self { listener, handler }
    }
}

impl<L, F, In> Listener<In> for Catch<L, F>
where
    In: Message + Sync,
    L: Listener<In>,
    L::Output: Sync,
    F: Fn(BoxError) -> Option<L::Output> + Send + Sync + 'static,
{
    type Output = L::Output;

    async fn listen(&self, event: &In) -> Result<Option<Self::Output>, BoxError> {
        match self.listener.listen(event).await {
            Ok(result) => Ok(result),
            Err(e) => Ok((self.handler)(e)),
        }
    }
}
