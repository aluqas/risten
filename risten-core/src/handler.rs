//! Handler trait for endpoint processing.

use crate::message::Message;
use std::future::Future;

/// A marker trait for the result of an endpoint execution.
pub trait HandlerResult: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> HandlerResult for T {}

/// A handler represents the final destination of an event processing pipeline.
///
/// It receives a fully owned message (Trigger) and performs async work.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot handle input of type `{In}`",
    label = "missing `Handler<{In}>` implementation",
    note = "Handlers must implement the `call` method for the input type `{In}`."
)]
pub trait Handler<In: Message>: Send + Sync + 'static {
    /// The output type of the handler, usually `()`, `Result`, or action type.
    type Output: HandlerResult;

    /// Executes the handler logic.
    fn call(&self, input: In) -> impl Future<Output = Self::Output> + Send;
}

// Blanket impl for closures
impl<F, In, Out, Fut> Handler<In> for F
where
    In: Message,
    Out: HandlerResult,
    F: Fn(In) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Out> + Send,
{
    type Output = Out;

    fn call(&self, input: In) -> impl Future<Output = Self::Output> + Send {
        (self)(input)
    }
}
