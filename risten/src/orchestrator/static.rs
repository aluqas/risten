//! Static dispatch layer for zero-cost hook chains.
//!
//! This module provides a HList-based implementation for compile-time
//! optimized hook dispatch. Hooks are chained at the type level, allowing
//! the compiler to inline and optimize the entire chain.

use crate::{
    core::{
        error::{BoxError, DispatchError},
        message::Message,
    },
    flow::hook::{Hook, HookResult},
    orchestrator::traits::Dispatcher,
};

/// HList terminator - represents an empty hook chain.
pub struct HNil;

/// HList cons cell - a hook followed by more hooks.
pub struct HCons<H, T> {
    pub head: H,
    pub tail: T,
}

/// Trait for dispatching events through a static hook chain.
///
/// Implemented for HList structures to provide compile-time optimized dispatch.
pub trait HookChain<E: Message>: Send + Sync + 'static {
    /// Dispatch an event through this chain.
    ///
    /// Returns the final `HookResult` after processing through all hooks
    /// or stopping early if a hook returns `HookResult::Stop`.
    fn dispatch_chain(
        &self,
        event: &E,
    ) -> impl std::future::Future<Output = Result<HookResult, BoxError>> + Send;
}

impl<E: Message> HookChain<E> for HNil {
    async fn dispatch_chain(&self, _event: &E) -> Result<HookResult, BoxError> {
        // End of chain, no more hooks to process
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

// ============================================================================
// Builder pattern for constructing HLists ergonomically
// ============================================================================

/// Builder for constructing static hook chains.
///
/// # Example
/// ```ignore
/// let chain = StaticChainBuilder::new()
///     .prepend(LoggingHook)
///     .prepend(MetricsHook)
///     .prepend(my_pipeline)
///     .build();
/// ```
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
    ///
    /// Note: Because this prepends to the HList, the execution order is reversed
    /// from the call order. Use the `static_hooks!` macro for intuitive ordering.
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

// ============================================================================
// Macro for convenient HList construction
// ============================================================================

/// Construct a static hook chain from a list of hooks.
///
/// # Example
/// ```ignore
/// let chain = static_hooks![LoggingHook, MetricsHook, my_pipeline];
/// ```
#[macro_export]
macro_rules! static_hooks {
    () => { $crate::HNil };
    ($hook:expr $(,)?) => {
        $crate::HCons {
            head: $hook,
            tail: $crate::HNil,
        }
    };
    ($hook:expr, $($rest:expr),+ $(,)?) => {
        $crate::HCons {
            head: $hook,
            tail: $crate::static_hooks!($($rest),+),
        }
    };
}

// ============================================================================
// Dispatcher using static chains
// ============================================================================

/// A dispatcher that uses a statically-typed hook chain.
///
/// This provides zero-cost abstraction as the entire dispatch chain
/// is known at compile time and can be fully inlined.
pub struct StaticDispatcher<C> {
    chain: C,
}

impl<C> StaticDispatcher<C> {
    /// Create a new static dispatcher with the given hook chain.
    pub fn new(chain: C) -> Self {
        Self { chain }
    }
}

impl<E, C> Dispatcher<E> for StaticDispatcher<C>
where
    E: Message + Sync,
    C: HookChain<E>,
{
    type Error = DispatchError;

    async fn dispatch(&self, event: E) -> Result<(), Self::Error> {
        self.chain.dispatch_chain(&event).await?;
        Ok(())
    }
}

// ============================================================================
// HList Length (simpler utility)
// ============================================================================

/// Trait for computing HList length at compile time.
pub trait HListLen {
    const LEN: usize;
}

impl HListLen for HNil {
    const LEN: usize = 0;
}

impl<H, T: HListLen> HListLen for HCons<H, T> {
    const LEN: usize = 1 + T::LEN;
}

#[cfg(test)]
mod tests {
    use super::{HCons, HListLen, HNil, StaticChainBuilder};

    // Basic compile-time tests
    #[test]
    fn test_hnil_creation() {
        let _: HNil = HNil;
    }

    #[test]
    fn test_hcons_creation() {
        let _: HCons<i32, HNil> = HCons {
            head: 42,
            tail: HNil,
        };
    }

    #[test]
    fn test_builder() {
        let chain = StaticChainBuilder::new()
            .prepend(1)
            .prepend(2)
            .prepend(3)
            .build();

        // Builder prepends to front, so order is reversed
        assert_eq!(chain.head, 3);
        assert_eq!(chain.tail.head, 2);
        assert_eq!(chain.tail.tail.head, 1);
    }

    #[test]
    fn test_static_hooks_macro() {
        let chain = static_hooks![1, 2, 3];
        // Macro preserves declaration order
        assert_eq!(chain.head, 1);
        assert_eq!(chain.tail.head, 2);
        assert_eq!(chain.tail.tail.head, 3);
    }

    #[test]
    fn test_hlist_len() {
        assert_eq!(<HNil as HListLen>::LEN, 0);
        assert_eq!(<HCons<i32, HNil> as HListLen>::LEN, 1);
        assert_eq!(<HCons<i32, HCons<i32, HNil>> as HListLen>::LEN, 2);
    }
}
