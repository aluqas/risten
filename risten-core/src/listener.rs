//! Listener trait for event transformation.

use crate::{handler::Handler, message::Message};

/// A listener sits at the entry or intermediate points of the event pipeline.
///
/// Its role is to inspect (borrow) an input event and optionally produce an output event
/// or decide to route it further. It is synchronous and lightweight.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Listener` for `{In}`",
    label = "missing `Listener` implementation",
    note = "Listeners must implement the `listen` method to process `{In}`."
)]
pub trait Listener<In: Message>: Send + Sync + 'static {
    /// The type of message this listener produces.
    type Output: Message;

    /// Inspects the input event and transforms it into the Output type if applicable.
    ///
    /// Returns `None` if the event should be ignored.
    fn listen(&self, event: &In) -> Option<Self::Output>;

    /// Chains this listener with another listener.
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

    /// Connects this listener to a handler (alias for consistency with older API).
    #[deprecated(note = "use `handler` instead")]
    fn endpoint<H>(self, handler: H) -> Pipeline<Self, H>
    where
        Self: Sized,
        H: crate::handler::Handler<Self::Output>,
    {
        self.handler(handler)
    }
}

/// A chain of two listeners.
pub struct Chain<A, B> {
    pub(crate) first: A,
    pub(crate) second: B,
}

impl<A, B, In> Listener<In> for Chain<A, B>
where
    In: Message,
    A: Listener<In>,
    B: Listener<A::Output>,
{
    type Output = B::Output;

    fn listen(&self, event: &In) -> Option<Self::Output> {
        let intermediate = self.first.listen(event)?;
        self.second.listen(&intermediate)
    }
}

/// A complete pipeline connecting a Listener to a Handler.
///
/// Implements `Hook` (in risten-std) so it can be registered with a dispatcher.
pub struct Pipeline<L, H> {
    /// The listener component.
    pub listener: L,
    /// The handler component.
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
        // Phase 1: Listener (Sync, Borrow)
        if let Some(out) = self.listener.listen(event) {
            // Phase 2: Handler (Async, Own)
            let result = self.handler.call(out).await;

            // Convert the handler's output to a hook outcome
            result.into_response()
        } else {
            Ok(HookResult::Next)
        }
    }
}
