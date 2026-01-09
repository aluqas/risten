use crate::{
    core::{error::DispatchError, message::Message},
    flow::hook::DynHook,
};
use std::{future::Future, pin::Pin};

/// Strategy for delivering an event to a resolved set of hooks.
///
/// This abstraction allows different execution models (sequential, parallel, etc.)
/// to be plugged into the dispatcher.
pub trait DeliveryStrategy: Send + Sync {
    /// Deliver the event to the hooks.
    fn deliver<'a, E, I>(
        &self,
        event: E,
        hooks: I,
    ) -> impl Future<Output = Result<(), DispatchError>> + Send
    where
        E: Message + Sync + 'a,
        I: Iterator<Item = &'a dyn DynHook<E>> + Send + 'a;
}

/// Object-safe version of `DeliveryStrategy`.
pub trait DynDeliveryStrategy: Send + Sync {
    /// Deliver the event to the hooks.
    fn deliver<'a, E, I>(
        &'a self,
        event: E,
        hooks: I,
    ) -> Pin<Box<dyn Future<Output = Result<(), DispatchError>> + Send + 'a>>
    where
        E: Message + Sync + 'a,
        I: Iterator<Item = &'a dyn DynHook<E>> + Send + 'a;
}

impl<T> DynDeliveryStrategy for T
where
    T: DeliveryStrategy,
{
    fn deliver<'a, E, I>(
        &'a self,
        event: E,
        hooks: I,
    ) -> Pin<Box<dyn Future<Output = Result<(), DispatchError>> + Send + 'a>>
    where
        E: Message + Sync + 'a,
        I: Iterator<Item = &'a dyn DynHook<E>> + Send + 'a,
    {
        Box::pin(self.deliver(event, hooks))
    }
}
