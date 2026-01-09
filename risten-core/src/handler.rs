//! # Context Layer (Handler)
//!
//! Wraps user-defined methods to inject framework-specific context (extractors,
//! error handling, response conversion). This is the terminal point of the
//! event processing pipeline.
//!
//! # Layer Position
//!
//! This is **Layer 4 (Context)** in the Risten architecture.
//! Handlers are the endpoint where business logic executes.
//!
//! # Design Philosophy
//!
//! - **Wrapper**: Adds event-architecture features (Context extraction, etc.)
//!   to plain user functions
//! - **Terminal**: The final destination; no further propagation after a Handler
//! - **Optional**: Users can implement `Handler` directly or use closures.
//!   For advanced context injection, use [`ExtractHandler`].
//!
//! # Usage Patterns
//!
//! 1. **Direct closure**: `|event| async move { ... }`
//! 2. **Struct implementation**: `impl Handler<MyEvent> for MyHandler`
//! 3. **Extractor-based**: `ExtractHandler::new(|ctx: UserContext| async { ... })`
//!
//! [`ExtractHandler`]: crate::ExtractHandler

use crate::message::Message;
use std::future::Future;

/// A marker trait for the result of an endpoint execution.
pub trait HandlerResult: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> HandlerResult for T {}

/// The terminal endpoint of an event processing pipeline.
///
/// Handlers receive a fully owned message and perform async business logic.
/// They represent the "action" phase where side effects occur.
///
/// # Layer Position
///
/// This is **Layer 4 (Context)** in the Risten architecture.
/// Connected to a [`Listener`] via [`Pipeline`], the combination becomes a [`Hook`].
///
/// [`Listener`]: crate::Listener
/// [`Pipeline`]: crate::Pipeline
/// [`Hook`]: crate::Hook
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
