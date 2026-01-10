//! # Context Layer (Handler)
//!
//! Wraps user-defined methods to inject framework-specific context.

use crate::message::Message;
use std::future::Future;
use std::pin::Pin;

/// A marker trait for the result of an endpoint execution.
pub trait HandlerResult: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> HandlerResult for T {}

/// The terminal endpoint of an event processing pipeline.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot handle input of type `{In}`",
    label = "missing `Handler<{In}>` implementation",
    note = "Handlers must implement the `call` method for the input type `{In}`."
)]
pub trait Handler<In: Message>: Send + Sync + 'static {
    type Output: HandlerResult;

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

/// Dynamic object-safe handler.
pub trait DynHandler<In: Message>: Send + Sync + 'static {
    type Output: HandlerResult;
    fn call_dyn<'a>(&'a self, input: In) -> Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>;
}

impl<H, In> DynHandler<In> for H
where
    H: Handler<In>,
    In: Message,
{
    type Output = H::Output;
    fn call_dyn<'a>(&'a self, input: In) -> Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>> {
        Box::pin(self.call(input))
    }
}
