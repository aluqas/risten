//! Static fan-out router.
//!
//! This module provides a parallel routing implementation for static hook chains.
//! Unlike `StaticRouter` which executes hooks sequentially, `StaticFanoutRouter`
//! executes all hooks in the chain concurrently.

use crate::static_dispatch::{HCons, HNil};
use futures::future::join;
use risten_core::{BoxError, DispatchError, Hook, HookResult, Message, Router};

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

/// A router that uses a statically-typed hook chain and executes them in parallel.
pub struct StaticFanoutRouter<C> {
    /// The hook chain.
    pub chain: C,
}

impl<C> StaticFanoutRouter<C> {
    /// Create a new static fanout router.
    pub fn new(chain: C) -> Self {
        Self { chain }
    }
}

impl<E, C> Router<E> for StaticFanoutRouter<C>
where
    E: Message + Sync + 'static,
    C: FanoutChain<E>,
{
    type Error = DispatchError;

    async fn route(&self, event: &E) -> Result<(), Self::Error> {
        self.chain
            .dispatch_fanout(event)
            .await
            .map_err(DispatchError::Listener)
    }
}

/// Macro to create a static fanout dispatcher chain key-value or just chain.
#[macro_export]
macro_rules! static_fanout {
    ($($args:tt)*) => {
        $crate::static_hooks!($($args)*)
    };
}
