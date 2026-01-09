use crate::{core::message::Message, flow::hook::DynHook};
use std::{future::Future, pin::Pin};

/// A provider of hooks for a given event.
///
/// This trait abstracts the source of hooks (e.g., a Registry or a Router).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid HookProvider for `{E}`",
    label = "missing `HookProvider` implementation",
    note = "Implement `HookProvider<{E}>` to allow resolving hooks for this event type."
)]
pub trait HookProvider<E: Message> {
    /// Resolve hooks for the given event.
    ///
    /// The iterator borrows from `self` (the provider), and is explicitly independent
    /// of the `event` lifetime to allow moving the event after resolution.
    fn resolve<'a>(&'a self, event: &E) -> Box<dyn Iterator<Item = &'a dyn DynHook<E>> + Send + 'a>
    where
        E: 'a;
}

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
        E: Message + 'a; // E must live as long as the future if captured, technically E is moved in so 'a might not be needed for E but for self
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
