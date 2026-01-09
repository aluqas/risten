//! Dynamic dispatch escape hatch for runtime flexibility.
//!
//! This module provides explicit wrappers for dynamic dispatch,
//! allowing runtime flexibility when needed within otherwise static pipelines.
//!
//! # Philosophy
//!
//! Static dispatch is the default in risten. Use these types **only when**:
//! - Plugin systems require runtime hook registration
//! - Hot-reloading scenarios
//! - The set of hooks is not known at compile time
//!
//! # Performance
//!
//! Using dynamic dispatch incurs:
//! - vtable lookup overhead
//! - Potential heap allocation
//! - Reduced inlining opportunities
//!
//! For most use cases, prefer static dispatch via `StaticDispatcher` or `enum_hook!`.

use crate::{
    core::{
        error::{BoxError, DispatchError},
        message::Message,
    },
    flow::hook::{Hook, HookResult},
    orchestrator::{
        delivery::traits::DeliveryStrategy,
        traits::{Dispatcher, HookProvider},
    },
};
use std::{future::Future, pin::Pin, sync::Arc};

/// A wrapper that enables dynamic dispatch for a Hook.
///
/// Use this when you need to insert a runtime-determined hook into a static chain.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{DynamicHook, static_hooks, StaticDispatcher};
///
/// // Create a dynamic hook from any Hook implementation
/// let dynamic = DynamicHook::new(MyRuntimeHook::new());
///
/// // Can be used in static chains
/// let chain = static_hooks![StaticHook1, dynamic, StaticHook2];
/// ```
pub struct DynamicHook<E: Message> {
    inner: Arc<dyn DynHookTrait<E>>,
}

/// Internal trait for type-erased hook dispatch.
trait DynHookTrait<E: Message>: Send + Sync + 'static {
    fn call<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<Box<dyn Future<Output = Result<HookResult, BoxError>> + Send + 'a>>;
}

impl<E: Message, H: Hook<E>> DynHookTrait<E> for H {
    fn call<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<Box<dyn Future<Output = Result<HookResult, BoxError>> + Send + 'a>> {
        Box::pin(self.on_event(event))
    }
}

impl<E: Message> DynamicHook<E> {
    /// Create a new dynamic hook wrapper.
    ///
    /// The hook is wrapped in an Arc for cheap cloning.
    pub fn new<H: Hook<E>>(hook: H) -> Self {
        Self {
            inner: Arc::new(hook),
        }
    }
}

impl<E: Message> Clone for DynamicHook<E> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<E: Message + Sync> Hook<E> for DynamicHook<E> {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        self.inner.call(event).await
    }
}

// ============================================================================
// DynRouter - Dynamic routing for plugin systems
// ============================================================================

/// A trait for routers that can be modified at runtime.
///
/// Unlike `Router` which is typically static, `DynRouter` allows
/// runtime registration and deregistration of routes.
///
/// # Example
///
/// ```rust,ignore
/// use risten::dynamic::MutableRouter;
///
/// let mut router = MutableRouter::new();
/// router.register("plugin.command", handler);
/// router.unregister("plugin.command");
/// ```
pub trait DynRouter<K: ?Sized, V>: Send + Sync {
    /// Register a new route.
    fn register(&mut self, key: K, value: V) -> Result<(), DynRouterError>
    where
        K: Sized;

    /// Unregister an existing route.
    fn unregister(&mut self, key: &K) -> Option<V>;

    /// Check if a route exists.
    fn contains(&self, key: &K) -> bool;

    /// Look up a route.
    fn get(&self, key: &K) -> Option<&V>;
}

/// Error type for dynamic router operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynRouterError {
    /// Route already exists.
    DuplicateRoute(String),
    /// Route not found.
    NotFound(String),
}

impl std::fmt::Display for DynRouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynRouterError::DuplicateRoute(key) => write!(f, "Duplicate route: {}", key),
            DynRouterError::NotFound(key) => write!(f, "Route not found: {}", key),
        }
    }
}

impl std::error::Error for DynRouterError {}

// ============================================================================
// MutableRouter - HashMap-based DynRouter implementation
// ============================================================================

use std::{collections::HashMap, hash::Hash};

/// A mutable router backed by HashMap for runtime modification.
pub struct MutableRouter<K, V> {
    routes: HashMap<K, V>,
}

impl<K, V> MutableRouter<K, V> {
    /// Create a new empty mutable router.
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Get the number of registered routes.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check if the router is empty.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

impl<K, V> Default for MutableRouter<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> DynRouter<K, V> for MutableRouter<K, V>
where
    K: Eq + Hash + ToString + Send + Sync,
    V: Send + Sync,
{
    fn register(&mut self, key: K, value: V) -> Result<(), DynRouterError> {
        if self.routes.contains_key(&key) {
            return Err(DynRouterError::DuplicateRoute(key.to_string()));
        }
        self.routes.insert(key, value);
        Ok(())
    }

    fn unregister(&mut self, key: &K) -> Option<V> {
        self.routes.remove(key)
    }

    fn contains(&self, key: &K) -> bool {
        self.routes.contains_key(key)
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.routes.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HookResult;

    #[derive(Clone)]
    struct TestEvent;

    struct TestHook(i32);

    impl Hook<TestEvent> for TestHook {
        async fn on_event(&self, _event: &TestEvent) -> Result<HookResult, BoxError> {
            Ok(HookResult::Next)
        }
    }

    #[tokio::test]
    async fn test_dynamic_hook_dispatch() {
        let hook1 = TestHook(1);
        let dynamic = DynamicHook::new(hook1);

        let result = dynamic.on_event(&TestEvent).await.unwrap();
        assert_eq!(result, HookResult::Next);
    }

    #[tokio::test]
    async fn test_dynamic_hook_clone() {
        let hook = TestHook(42);
        let dynamic1 = DynamicHook::new(hook);
        let dynamic2 = dynamic1.clone();

        // Both should work
        assert_eq!(
            dynamic1.on_event(&TestEvent).await.unwrap(),
            HookResult::Next
        );
        assert_eq!(
            dynamic2.on_event(&TestEvent).await.unwrap(),
            HookResult::Next
        );
    }

    #[test]
    fn test_mutable_router() {
        let mut router: MutableRouter<&str, i32> = MutableRouter::new();

        assert!(router.is_empty());

        router.register("foo", 1).unwrap();
        router.register("bar", 2).unwrap();

        assert_eq!(router.len(), 2);
        assert!(router.contains(&"foo"));
        assert_eq!(router.get(&"foo"), Some(&1));

        // Duplicate should fail
        assert!(router.register("foo", 3).is_err());

        // Unregister
        assert_eq!(router.unregister(&"foo"), Some(1));
        assert!(!router.contains(&"foo"));
    }
}
// ============================================================================
// DynamicDispatcher
// ============================================================================

/// A dynamic dispatcher that uses trait objects for runtime flexibility.
///
/// This dispatcher resolves hooks at runtime via a `HookProvider` and executes
/// them using a `DeliveryStrategy`. While flexible, this incurs the cost of
/// dynamic dispatch (vtable lookups).
///
/// # When to use
///
/// - Plugin systems requiring runtime registration
/// - Hot-reloading scenarios
/// - When the hook set changes during runtime
///
/// # Performance note
///
/// For static, compile-time known hook chains, prefer `StaticDispatcher` or
/// `StaticFanoutDispatcher` which eliminate vtable overhead entirely.
pub struct DynamicDispatcher<P, D> {
    provider: P,
    delivery: D,
}

impl<P, D> DynamicDispatcher<P, D> {
    /// Create a new dynamic dispatcher.
    pub fn new(provider: P, delivery: D) -> Self {
        Self { provider, delivery }
    }

    /// Get a reference to the provider.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Get a reference to the delivery strategy.
    pub fn delivery(&self) -> &D {
        &self.delivery
    }
}

// [trait_variant] generates DynDispatcher trait which is object-safe.
// We implement the main Dispatcher trait here.
impl<E, P, D> Dispatcher<E> for DynamicDispatcher<P, D>
where
    E: Message + Sync + 'static,
    P: HookProvider<E> + Send + Sync,
    D: DeliveryStrategy + Send + Sync,
{
    type Error = DispatchError;

    async fn dispatch(&self, event: E) -> Result<(), DispatchError> {
        // 1. Resolve hooks from the provider
        let hooks = self.provider.resolve(&event);

        // 2. Delegate execution to the delivery strategy
        self.delivery.deliver(event, hooks).await
    }
}
