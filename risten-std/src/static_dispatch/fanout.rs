//! Static fan-out router.
//!
//! This module provides a parallel routing implementation for static hook chains.
//! Unlike `StaticRouter` which executes hooks sequentially, `StaticFanoutRouter`
//! executes all hooks in the chain concurrently.

use crate::static_dispatch::{HCons, HNil};
use futures::future::join;
use risten_core::{BoxError, RoutingError, Hook, HookResult, Message, RouteResult, Router};

/// Result of fanout dispatch including stop tracking.
pub struct FanoutResult {
    /// Whether any hook returned Stop.
    pub stopped: bool,
}

/// Trait for dispatching events through a static hook chain concurrently.
pub trait FanoutChain<E: Message>: Send + Sync + 'static {
    /// Dispatch an event through this chain concurrently.
    fn dispatch_fanout(
        &self,
        event: &E,
    ) -> impl std::future::Future<Output = Result<FanoutResult, BoxError>> + Send;
}

impl<E: Message> FanoutChain<E> for HNil {
    async fn dispatch_fanout(&self, _event: &E) -> Result<FanoutResult, BoxError> {
        Ok(FanoutResult { stopped: false })
    }
}

impl<E, H, T> FanoutChain<E> for HCons<H, T>
where
    E: Message + Sync + 'static,
    H: Hook<E>,
    T: FanoutChain<E>,
{
    async fn dispatch_fanout(&self, event: &E) -> Result<FanoutResult, BoxError> {
        let head_fut = self.head.on_event(event);
        let tail_fut = self.tail.dispatch_fanout(event);

        let (head_res, tail_res) = join(head_fut, tail_fut).await;

        let head_stopped = match head_res {
            Ok(HookResult::Stop) => true,
            Ok(HookResult::Next) => false,
            Err(e) => return Err(e),
        };

        let tail_result = tail_res?;

        Ok(FanoutResult {
            stopped: head_stopped || tail_result.stopped,
        })
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
    type Error = RoutingError;

    async fn route(&self, event: &E) -> Result<RouteResult, Self::Error> {
        let result = self
            .chain
            .dispatch_fanout(event)
            .await
            .map_err(RoutingError::Listener)?;
        Ok(RouteResult {
            stopped: result.stopped,
            executed_count: 0, // Fanout doesn't track count
        })
    }
}

/// Macro to create a static fanout dispatcher chain key-value or just chain.
#[macro_export]
macro_rules! static_fanout {
    ($($args:tt)*) => {
        $crate::static_hooks!($($args)*)
    };
}
