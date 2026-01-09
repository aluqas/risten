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

    /// Connects this listener to a handler, creating a complete pipeline.
    ///
    /// The resulting `Pipeline` implements `Hook` and can be registered with a dispatcher.
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
        // First listener processes the event
        let Some(intermediate) = self.first.listen(event).await? else {
            return Ok(None);
        };
        // Second listener processes the intermediate result
        self.second.listen(&intermediate).await
    }
}

/// A complete pipeline connecting a Listener to a Handler.
///
/// Implements [`Hook`] so it can be registered with a dispatcher.
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
        // Phase 1: Listener (Async, Gatekeeping/Transformation)
        match self.listener.listen(event).await {
            Ok(Some(out)) => {
                // Phase 2: Handler (Async, Business Logic)
                let result = self.handler.call(out).await;
                result.into_response()
            }
            Ok(None) => {
                // Event was filtered out, continue to next hook
                Ok(HookResult::Next)
            }
            Err(e) => Err(e),
        }
    }
}
