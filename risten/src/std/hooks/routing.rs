//! RoutingHook - Router as a Hook in the pipeline.
//!
//! This module provides `RoutingHook`, which wraps a `Router` to enable
//! routing within a Hook chain. Events are routed to sub-dispatchers
//! based on a key extracted from the event.
//!
//! # Example
//!
//! ```rust,ignore
//! use risten::{RoutingHook, HashMapRouter, StaticDispatcher};
//!
//! let routing_hook = RoutingHook::new(router, |event: &CommandEvent| {
//!     Some(event.command_name.as_str())
//! });
//! ```

use crate::{
    core::{
        error::{BoxError, DispatchError},
        message::Message,
    },
    flow::{
        hook::{Hook, HookResult},
        routing::{RouteResult, Router},
    },
    orchestrator::traits::{Dispatcher, DynDispatcher},
};
use std::marker::PhantomData;

// ============================================================================
// KeyExtractor Trait
// ============================================================================

/// Extracts a routing key from an event.
///
/// This trait allows flexible key extraction strategies - from simple
/// field access to complex parsing logic.
///
/// # Example
///
/// ```rust,ignore
/// // Using a closure (blanket impl)
/// let extractor = |event: &MyEvent| Some(event.name.as_str());
///
/// // Or implement the trait directly for complex logic
/// struct CommandExtractor;
/// impl KeyExtractor<MyEvent> for CommandExtractor {
///     type Key = String;
///     fn extract(&self, event: &MyEvent) -> Option<String> {
///         event.content.split_whitespace().next().map(String::from)
///     }
/// }
/// ```
pub trait KeyExtractor<E: Message>: Send + Sync + 'static {
    /// The type of key extracted from events.
    type Key: Send + Sync;

    /// Extract a routing key from the event.
    ///
    /// Returns `None` if the event should not be routed (e.g., doesn't match expected format).
    fn extract(&self, event: &E) -> Option<Self::Key>;
}

// Blanket implementation for closures
impl<E, K, F> KeyExtractor<E> for F
where
    E: Message,
    K: Send + Sync,
    F: Fn(&E) -> Option<K> + Send + Sync + 'static,
{
    type Key = K;

    fn extract(&self, event: &E) -> Option<K> {
        (self)(event)
    }
}

// ============================================================================
// RoutingHook
// ============================================================================

/// A Hook that routes events to sub-dispatchers based on extracted keys.
///
/// `RoutingHook` bridges the gap between the `Router` abstraction and the
/// Hook-based pipeline. It extracts a key from incoming events, looks up
/// the corresponding dispatcher in the router, and executes it.
///
/// # Type Parameters
///
/// - `R`: The router type (e.g., `HashMapRouter`, `PhfRouter`)
/// - `F`: The key extractor (closure or `KeyExtractor` impl)
/// - `E`: The event type
///
/// # Behavior
///
/// - If key extraction returns `None`: returns `HookResult::Next`
/// - If route is found: dispatches to the matched dispatcher, returns `HookResult::Stop`
/// - If route is not found and fallback exists: dispatches to fallback, returns `HookResult::Stop`
/// - If route is not found and no fallback: returns `HookResult::Next`
///
/// # Example
///
/// ```rust,ignore
/// use risten::{RoutingHook, HashMapRouterBuilder, StaticDispatcher, static_hooks};
///
/// // Build router with dispatchers
/// let mut builder = HashMapRouterBuilder::new();
/// builder.insert("ping", Box::new(ping_dispatcher) as Box<dyn DynDispatcher<_>>);
/// builder.insert("echo", Box::new(echo_dispatcher) as Box<dyn DynDispatcher<_>>);
/// let router = builder.build().unwrap();
///
/// // Create routing hook
/// let routing = RoutingHook::new(router, |e: &CommandEvent| {
///     Some(e.command.as_str())
/// });
///
/// // Use in a static chain
/// let chain = static_hooks![LoggingHook, routing, FallbackHook];
/// ```
pub struct RoutingHook<R, F, E>
where
    E: Message,
{
    router: R,
    extractor: F,
    fallback: Option<Box<dyn DynDispatcher<E, Error = DispatchError>>>,
    /// Whether to stop propagation after successful routing.
    stop_on_match: bool,
    _marker: PhantomData<fn() -> E>,
}

impl<R, F, E> RoutingHook<R, F, E>
where
    E: Message,
    F: KeyExtractor<E>,
{
    /// Create a new `RoutingHook` with the given router and key extractor.
    pub fn new(router: R, extractor: F) -> Self {
        Self {
            router,
            extractor,
            fallback: None,
            stop_on_match: true,
            _marker: PhantomData,
        }
    }

    /// Set a fallback dispatcher for unmatched routes.
    pub fn with_fallback<D>(mut self, fallback: D) -> Self
    where
        D: Dispatcher<E, Error = DispatchError> + Send + Sync + 'static,
    {
        self.fallback = Some(Box::new(fallback));
        self
    }

    /// Configure whether to stop propagation after a successful match.
    ///
    /// Default is `true` (stop after routing).
    pub fn stop_on_match(mut self, stop: bool) -> Self {
        self.stop_on_match = stop;
        self
    }
}

impl<R, F, E, K> Hook<E> for RoutingHook<R, F, E>
where
    E: Message + Clone + Sync,
    F: KeyExtractor<E, Key = K>,
    K: Send + Sync,
    R: Router<K, Box<dyn DynDispatcher<E, Error = DispatchError>>> + Send + Sync + 'static,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        // 1. Extract the routing key
        let key = match self.extractor.extract(event) {
            Some(k) => k,
            None => return Ok(HookResult::Next), // No key → skip routing
        };

        // 2. Look up the route
        match self.router.route(&key) {
            RouteResult::Matched(dispatcher) => {
                // 3. Dispatch to the matched handler
                dispatcher.dispatch(event.clone()).await?;

                if self.stop_on_match {
                    Ok(HookResult::Stop)
                } else {
                    Ok(HookResult::Next)
                }
            }
            RouteResult::NotFound => {
                // 4. Try fallback if available
                if let Some(ref fallback) = self.fallback {
                    fallback.dispatch(event.clone()).await?;
                    Ok(HookResult::Stop)
                } else {
                    Ok(HookResult::Next) // No route, no fallback → continue
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{flow::routing::RouterBuilder, std::routing::hashmap::HashMapRouterBuilder};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone, Debug)]
    struct TestEvent {
        command: String,
    }

    struct CountingDispatcher {
        count: Arc<AtomicUsize>,
    }

    impl Dispatcher<TestEvent> for CountingDispatcher {
        type Error = crate::core::error::DispatchError;
        async fn dispatch(&self, _event: TestEvent) -> Result<(), Self::Error> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_routing_hook_matches() {
        let ping_count = Arc::new(AtomicUsize::new(0));
        let echo_count = Arc::new(AtomicUsize::new(0));

        let mut builder: HashMapRouterBuilder<
            String,
            Box<dyn DynDispatcher<TestEvent, Error = DispatchError>>,
        > = HashMapRouterBuilder::default();
        builder
            .insert(
                "ping".to_string(),
                Box::new(CountingDispatcher {
                    count: Arc::clone(&ping_count),
                }),
            )
            .unwrap();
        builder
            .insert(
                "echo".to_string(),
                Box::new(CountingDispatcher {
                    count: Arc::clone(&echo_count),
                }),
            )
            .unwrap();
        let router = builder.build().unwrap();

        let hook = RoutingHook::new(router, |e: &TestEvent| Some(e.command.clone()));

        // Test ping
        let event = TestEvent {
            command: "ping".into(),
        };
        let result = hook.on_event(&event).await.unwrap();
        assert_eq!(result, HookResult::Stop);
        assert_eq!(ping_count.load(Ordering::SeqCst), 1);
        assert_eq!(echo_count.load(Ordering::SeqCst), 0);

        // Test echo
        let event = TestEvent {
            command: "echo".into(),
        };
        let result = hook.on_event(&event).await.unwrap();
        assert_eq!(result, HookResult::Stop);
        assert_eq!(ping_count.load(Ordering::SeqCst), 1);
        assert_eq!(echo_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_routing_hook_not_found() {
        let builder: HashMapRouterBuilder<
            String,
            Box<dyn DynDispatcher<TestEvent, Error = DispatchError>>,
        > = HashMapRouterBuilder::default();
        let router = builder.build().unwrap();

        let hook = RoutingHook::new(router, |e: &TestEvent| Some(e.command.clone()));

        let event = TestEvent {
            command: "unknown".into(),
        };
        let result = hook.on_event(&event).await.unwrap();
        assert_eq!(result, HookResult::Next); // No match, no fallback → Next
    }

    #[tokio::test]
    async fn test_routing_hook_with_fallback() {
        let fallback_count = Arc::new(AtomicUsize::new(0));

        let builder: HashMapRouterBuilder<
            String,
            Box<dyn DynDispatcher<TestEvent, Error = DispatchError>>,
        > = HashMapRouterBuilder::default();
        let router = builder.build().unwrap();

        let hook = RoutingHook::new(router, |e: &TestEvent| Some(e.command.clone())).with_fallback(
            CountingDispatcher {
                count: Arc::clone(&fallback_count),
            },
        );

        let event = TestEvent {
            command: "unknown".into(),
        };
        let result = hook.on_event(&event).await.unwrap();
        assert_eq!(result, HookResult::Stop); // Fallback was called
        assert_eq!(fallback_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_routing_hook_no_key() {
        let builder: HashMapRouterBuilder<
            String,
            Box<dyn DynDispatcher<TestEvent, Error = DispatchError>>,
        > = HashMapRouterBuilder::default();
        let router = builder.build().unwrap();

        // Extractor that always returns None
        let hook = RoutingHook::new(router, |_: &TestEvent| -> Option<String> { None });

        let event = TestEvent {
            command: "ping".into(),
        };
        let result = hook.on_event(&event).await.unwrap();
        assert_eq!(result, HookResult::Next); // No key → skip
    }
}
