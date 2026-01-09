//! Listener trait for event transformation.
//!
//! Listeners are the high-level abstraction for event processing in Risten.
//! Unlike the low-level [`Hook`] trait which operates as a primitive middleware,
//! Listeners provide a richer API for:
//! - **Gatekeeping**: Deciding whether an event should be processed further
//! - **Transformation**: Converting events into different forms
//!
//! ## Design Philosophy
//!
//! - **Listener** (Policy): Interprets event meaning, filters, transforms. No side effects.
//! - **Handler** (Action): Performs business logic and side effects. Terminal.
//! - **Hook** (Mechanism): Low-level control flow manipulation.
//!
//! ## Declarative Pipeline Construction
//!
//! Listeners support method chaining for building pipelines:
//!
//! ```rust,ignore
//! let pipeline = AuthListener
//!     .filter(|e| !e.author.is_bot)           // Gatekeeping
//!     .then(|e| async move { e.load_ctx() })  // Async enrichment
//!     .map(|ctx| CommandContext::from(ctx))   // Transformation
//!     .handler(CommandHandler);
//! ```

use crate::{error::BoxError, handler::Handler, message::Message};
use std::future::Future;

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
}

// ============================================================================
// Chain (and_then)
// ============================================================================

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

// ============================================================================
// Filter
// ============================================================================

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

// ============================================================================
// Map
// ============================================================================

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

// ============================================================================
// Then (async map)
// ============================================================================

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

// ============================================================================
// FilterMap
// ============================================================================

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

// ============================================================================
// Pipeline (Listener + Handler)
// ============================================================================

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
