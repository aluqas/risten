//! Dispatcher core traits.

use crate::message::Message;
use std::{future::Future, pin::Pin};

/// A dispatcher that executes hooks for an event.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot dispatch events of type `{E}`",
    label = "missing `Dispatcher` implementation",
    note = "Implement `Dispatcher<{E}>` to handle event dispatching."
)]
pub trait Dispatcher<E: Message>: Send + Sync {
    /// The error type returned by dispatch operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Dispatch the event to the registered hooks.
    fn dispatch(&self, event: E) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// Object-safe version of `Dispatcher` for dynamic dispatch.
pub trait DynDispatcher<E>: Send + Sync {
    /// The error type returned by dispatch operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Dispatch the event to the registered hooks.
    fn dispatch<'a>(
        &'a self,
        event: E,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        E: Message + 'a;
}

impl<T, E> DynDispatcher<E> for T
where
    T: Dispatcher<E>,
    E: Message,
{
    type Error = T::Error;

    fn dispatch<'a>(
        &'a self,
        event: E,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>
    where
        E: Message + 'a,
    {
        Box::pin(self.dispatch(event))
    }
}
