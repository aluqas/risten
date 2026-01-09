//! Static dispatch layer for zero-cost hook chains.
//!
//! This module provides HList-based implementation for compile-time
//! optimized hook dispatch.

use risten_core::{BoxError, RoutingError, Hook, HookResult, Message, RouteResult, Router};

/// HList terminator - represents an empty hook chain.
pub struct HNil;

/// HList cons cell - a hook followed by more hooks.
pub struct HCons<H, T> {
    /// The head hook.
    pub head: H,
    /// The tail of the chain.
    pub tail: T,
}

pub mod fanout;

pub use fanout::{FanoutChain, StaticFanoutRouter};

/// Trait for dispatching events through a static hook chain.
pub trait HookChain<E: Message>: Send + Sync + 'static {
    /// Dispatch an event through this chain.
    fn dispatch_chain(
        &self,
        event: &E,
    ) -> impl std::future::Future<Output = Result<HookResult, BoxError>> + Send;
}

impl<E: Message> HookChain<E> for HNil {
    async fn dispatch_chain(&self, _event: &E) -> Result<HookResult, BoxError> {
        Ok(HookResult::Next)
    }
}

impl<E, H, T> HookChain<E> for HCons<H, T>
where
    E: Message + Sync,
    H: Hook<E>,
    T: HookChain<E>,
{
    async fn dispatch_chain(&self, event: &E) -> Result<HookResult, BoxError> {
        match self.head.on_event(event).await? {
            HookResult::Stop => Ok(HookResult::Stop),
            HookResult::Next => self.tail.dispatch_chain(event).await,
        }
    }
}

/// Builder for constructing static hook chains.
pub struct StaticChainBuilder<T> {
    chain: T,
}

impl StaticChainBuilder<HNil> {
    /// Create a new empty chain builder.
    pub fn new() -> Self {
        Self { chain: HNil }
    }
}

impl Default for StaticChainBuilder<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> StaticChainBuilder<T> {
    /// Add a hook to the front of the chain.
    pub fn prepend<H>(self, hook: H) -> StaticChainBuilder<HCons<H, T>> {
        StaticChainBuilder {
            chain: HCons {
                head: hook,
                tail: self.chain,
            },
        }
    }

    /// Finalize and return the built hook chain.
    pub fn build(self) -> T {
        self.chain
    }
}

/// A router that uses a statically-typed hook chain.
///
/// This provides zero-cost abstraction as the entire routing chain
/// is known at compile time and can be fully inlined.
pub struct StaticRouter<C> {
    chain: C,
}

impl<C> StaticRouter<C> {
    /// Create a new static router with the given hook chain.
    pub fn new(chain: C) -> Self {
        Self { chain }
    }
}

impl<E, C> Router<E> for StaticRouter<C>
where
    E: Message + Sync + 'static,
    C: HookChain<E>,
{
    type Error = RoutingError;

    async fn route(&self, event: &E) -> Result<RouteResult, Self::Error> {
        let result = self
            .chain
            .dispatch_chain(event)
            .await
            .map_err(RoutingError::Listener)?;

        Ok(RouteResult {
            stopped: result == HookResult::Stop,
        })
    }
}

// Router as Listener (Native Integration)
//
// When a Router acts as a Listener, its routing result determines the output:
// - `stopped = true` (a hook consumed the event) → `None` (event handled, skip downstream)
// - `stopped = false` (event passed through) → `Some(event)` (continue pipeline)
use risten_core::Listener;

impl<C, E> Listener<E> for StaticRouter<C>
where
    E: Message + Sync + Clone + 'static,
    C: HookChain<E>,
{
    type Output = E;

    async fn listen(&self, event: &E) -> Result<Option<Self::Output>, BoxError> {
        let result = Router::route(self, event)
            .await
            .map_err(|e| Box::new(e) as BoxError)?;

        if result.stopped {
            Ok(None)
        } else {
            Ok(Some(event.clone()))
        }
    }
}

/// Trait for computing HList length at compile time.
pub trait HListLen {
    /// The length of this HList.
    const LEN: usize;
}

impl HListLen for HNil {
    const LEN: usize = 0;
}

impl<H, T: HListLen> HListLen for HCons<H, T> {
    const LEN: usize = 1 + T::LEN;
}

/// Construct a static hook chain from a list of hooks.
///
/// # Example
/// ```ignore
/// let chain = static_hooks![LoggingHook, MetricsHook, my_pipeline];
/// ```
#[macro_export]
macro_rules! static_hooks {
    () => { $crate::static_dispatch::HNil };
    ($hook:expr $(,)?) => {
        $crate::static_dispatch::HCons {
            head: $hook,
            tail: $crate::static_dispatch::HNil,
        }
    };
    ($hook:expr, $($rest:expr),+ $(,)?) => {
        $crate::static_dispatch::HCons {
            head: $hook,
            tail: $crate::static_hooks!($($rest),+),
        }
    };
}
