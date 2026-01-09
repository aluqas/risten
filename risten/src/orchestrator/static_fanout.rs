//! Static fan-out dispatcher.
//!
//! This module provides a parallel dispatch implementation for static hook chains.
//! Unlike `StaticDispatcher` which executes hooks sequentially, `StaticFanoutDispatcher`
//! executes all hooks in the chain concurrently.

use crate::{
    core::{
        error::{BoxError, DispatchError},
        message::Message,
    },
    flow::hook::{Hook, HookResult},
    orchestrator::{
        r#static::{HCons, HNil},
        traits::Dispatcher,
    },
};
use futures::future::join;

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
    E: Message + Sync + 'static, // 'static strictly required for Join in async recursion often
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
/// Reuses the structure of static_hooks! since the underlying HList is the same.
/// This macro exists mainly for semantic clarity.
#[macro_export]
macro_rules! static_fanout {
    ($($args:tt)*) => {
        $crate::static_hooks!($($args)*)
    };
}

#[cfg(test)]
mod tests {
    use super::StaticFanoutDispatcher;
    use crate::{
        Dispatcher,
        core::error::BoxError,
        flow::hook::{Hook, HookResult},
    };
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tokio::time::sleep;

    #[derive(Clone, Debug)]
    struct TestEvent;

    struct SlowHook {
        val: usize,
        out: Arc<AtomicUsize>,
    }

    impl Hook<TestEvent> for SlowHook {
        async fn on_event(&self, _event: &TestEvent) -> Result<HookResult, BoxError> {
            sleep(Duration::from_millis(50)).await;
            self.out.fetch_add(self.val, Ordering::SeqCst);
            Ok(HookResult::Next)
        }
    }

    #[tokio::test]
    async fn test_static_fanout_parallel() {
        let counter = Arc::new(AtomicUsize::new(0));

        // 3 hooks, each takes 50ms.
        // Sequential would take 150ms.
        // Parallel should take ~50ms.
        let h1 = SlowHook {
            val: 1,
            out: counter.clone(),
        };
        let h2 = SlowHook {
            val: 10,
            out: counter.clone(),
        };
        let h3 = SlowHook {
            val: 100,
            out: counter.clone(),
        };

        let chain = crate::static_hooks![h1, h2, h3];
        let dispatcher = StaticFanoutDispatcher::new(chain);

        let start = std::time::Instant::now();
        dispatcher.dispatch(TestEvent).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(counter.load(Ordering::SeqCst), 111);

        // Allow a little overhead, but ensure it's much faster than sequential (150ms)
        assert!(
            elapsed.as_millis() < 100,
            "Should be parallel, took {}ms",
            elapsed.as_millis()
        );
    }
}
