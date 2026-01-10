//! Dispatch-based router using inventory collection.

use risten_core::{Message, RouteResult, Router, DynHandler, ExtractError};
use futures::future::join_all;
use thiserror::Error;
use std::any::{Any, TypeId};
use std::pin::Pin;
use std::future::Future;

/// Type-erased handler trait.
pub trait ErasedHandler: Send + Sync {
    fn call_erased<'a>(&'a self, event: &'a (dyn Any + Send + Sync)) -> Pin<Box<dyn Future<Output = Result<(), ExtractError>> + Send + 'a>>;
}

/// Wrapper to implement ErasedHandler for a typed handler.
pub struct ErasedHandlerWrapper<E, H> {
    pub handler: H,
    pub _phantom: std::marker::PhantomData<E>,
}

impl<E, H> ErasedHandlerWrapper<E, H> {
    pub const fn new(handler: H) -> Self {
        Self { handler, _phantom: std::marker::PhantomData }
    }
}

impl<E, H> ErasedHandler for ErasedHandlerWrapper<E, H>
where
    E: Message + Clone + 'static,
    H: DynHandler<E, Output = Result<(), ExtractError>> + Send + Sync,
{
    fn call_erased<'a>(&'a self, event: &'a (dyn Any + Send + Sync)) -> Pin<Box<dyn Future<Output = Result<(), ExtractError>> + Send + 'a>> {
        let event_ref = event.downcast_ref::<E>().expect("Type mismatch in ErasedHandler");
        let event_owned = event_ref.clone();
        self.handler.call_dyn(event_owned)
    }
}

/// Global registration structure.
pub struct HandlerRegistration {
    pub type_id: TypeId,
    pub handler: &'static (dyn ErasedHandler + Send + Sync),
    pub priority: i32,
}

inventory::collect!(HandlerRegistration);

/// Errors occurring during dispatch.
#[derive(Debug, Error)]
pub enum DispatchError {
    #[error(transparent)]
    Extract(#[from] ExtractError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A router that executes all handlers collected via inventory for event `E`.
pub struct DispatchRouter<E> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E> DispatchRouter<E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> Default for DispatchRouter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Router<E> for DispatchRouter<E>
where
    E: Message + Clone + 'static,
{
    type Error = DispatchError;

    async fn route(&self, event: &E) -> Result<RouteResult, Self::Error> {
        let target_type = TypeId::of::<E>();
        let any_event = event as &(dyn Any + Send + Sync);

        let futures: Vec<_> = inventory::iter::<HandlerRegistration>()
             .into_iter()
             .filter(|reg| reg.type_id == target_type)
             .map(|reg| {
                reg.handler.call_erased(any_event)
            })
            .collect();

        if futures.is_empty() {
             return Ok(RouteResult::continued());
        }

        let results = join_all(futures).await;

        for res in results {
            if let Err(e) = res {
                return Err(DispatchError::Extract(e));
            }
        }

        Ok(RouteResult::continued())
    }
}
