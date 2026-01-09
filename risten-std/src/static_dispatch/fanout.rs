//! Static fan-out dispatcher.
//!
//! This module provides a parallel dispatch implementation for static hook chains.
//! Unlike `StaticDispatcher` which executes hooks sequentially, `StaticFanoutDispatcher`
//! executes all hooks in the chain concurrently.

use crate::static_dispatch::{HCons, HNil};
use futures::future::join;
use risten_core::{BoxError, DispatchError, Dispatcher, Hook, HookResult, Message};

/// Trait for dispatching events through a static hook chain concurrently.
pub trait FanoutChain<E: Message>: Send + Sync + 'static {
    /// Dispatch an event through this chain concurrently.
    fn dispatch_fanout(
        &self,
        event: &E,
    ) -> impl std::future::Future<Output = Result<(), BoxError>> + Send;
}

impl<E: Message> FanoutChain<E> for HNil {
    async fn dispatch_fanout(&self, _event: &E) -> Result<(), BoxError> {
        Ok(())
    }
}

impl<E, H, T> FanoutChain<E> for HCons<H, T>
where
    E: Message + Sync + 'static,
    H: Hook<E>,
    T: FanoutChain<E>,
{
    async fn dispatch_fanout(&self, event: &E) -> Result<(), BoxError> {
        // Start head and tail concurrently
        let head_fut = self.head.on_event(event);
        let tail_fut = self.tail.dispatch_fanout(event);

        // Wait for both to complete
        let (head_res, tail_res) = join(head_fut, tail_fut).await;

        // Check results
        match head_res {
            Ok(HookResult::Stop) => {
                // In fanout, Stop signifies "I handled it", but others ran anyway.
                // We just note it (or ignore it) for purely parallel fire-and-forget.
                // For this implementation, we simply propagate errors if any.
            }
            Ok(HookResult::Next) => {}
            Err(e) => return Err(e),
        }

        tail_res
    }
}

/// A dispatcher that uses a statically-typed hook chain and executes them in parallel.
pub struct StaticFanoutDispatcher<C> {
    pub chain: C,
}

impl<C> StaticFanoutDispatcher<C> {
    /// Create a new static fanout dispatcher.
    pub fn new(chain: C) -> Self {
        Self { chain }
    }
}

impl<E, C> Dispatcher<E> for StaticFanoutDispatcher<C>
where
    E: Message + Sync + 'static,
    C: FanoutChain<E>,
{
    type Error = DispatchError;

    async fn dispatch(&self, event: E) -> Result<(), Self::Error> {
        self.chain
            .dispatch_fanout(&event)
            .await
            .map_err(DispatchError::ListenerError)
    }
}

/// Macro to create a static fanout dispatcher chain key-value or just chain.
#[macro_export]
macro_rules! static_fanout {
    ($($args:tt)*) => {
        $crate::static_hooks!($($args)*)
    };
}
