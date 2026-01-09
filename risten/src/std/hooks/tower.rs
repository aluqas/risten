//! Tower integration for risten.
//!
//! This module provides adapters for interoperability between
//! risten's `Hook` trait and tower's `Service` trait.
//!
//! # Overview
//!
//! Tower is a library of modular and reusable components for building
//! robust networking clients and servers. This module allows you to:
//!
//! - Use risten Hooks as tower Services
//! - Use tower Services as risten Hooks
//! - Apply tower Layers to risten Hooks
//!
//! # Example
//!
//! ```rust,ignore
//! use risten::tower::{HookService, ServiceHook};
//!
//! // Wrap a Hook as a Service
//! let service = HookService::new(my_hook);
//!
//! // Wrap a Service as a Hook
//! let hook = ServiceHook::new(my_service);
//! ```

use crate::{
    error::BoxError,
    model::{Hook, HookResult, Message},
};
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;

// ============================================================================
// Hook → Service Adapter
// ============================================================================

/// Wraps a risten `Hook` as a tower `Service`.
///
/// This adapter allows any `Hook<E>` to be used where a
/// `tower::Service<E>` is expected.
///
/// # Type Parameters
///
/// - `H`: The Hook type to wrap
/// - `E`: The event type
///
/// # Example
///
/// ```rust,ignore
/// use risten::tower::HookService;
///
/// let hook = MyLoggingHook;
/// let service = HookService::new(hook);
///
/// // Now usable with tower middleware
/// let with_timeout = tower::timeout::Timeout::new(service, Duration::from_secs(5));
/// ```
pub struct HookService<H, E> {
    hook: Arc<H>,
    _marker: PhantomData<E>,
}

impl<H, E> HookService<H, E> {
    /// Create a new `HookService` wrapping the given hook.
    pub fn new(hook: H) -> Self {
        Self {
            hook: Arc::new(hook),
            _marker: PhantomData,
        }
    }

    /// Get a reference to the inner hook.
    pub fn inner(&self) -> &H {
        &self.hook
    }
}

impl<H, E> Clone for HookService<H, E> {
    fn clone(&self) -> Self {
        Self {
            hook: Arc::clone(&self.hook),
            _marker: PhantomData,
        }
    }
}

impl<H, E> ::tower::Service<E> for HookService<H, E>
where
    H: Hook<E> + 'static,
    E: Message + Clone,
{
    type Response = HookResult;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Hooks are always ready
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: E) -> Self::Future {
        let hook = Arc::clone(&self.hook);
        Box::pin(async move { hook.on_event(&request).await })
    }
}

// ============================================================================
// Service → Hook Adapter
// ============================================================================

/// Wraps a tower `Service` as a risten `Hook`.
///
/// This adapter allows any `tower::Service<E>` to be used where a
/// `Hook<E>` is expected, enabling integration of tower middleware
/// into risten pipelines.
///
/// # Type Parameters
///
/// - `S`: The Service type to wrap
///
/// # Requirements
///
/// The wrapped service must:
/// - Return `HookResult` as its `Response` type
/// - Return `BoxError` as its `Error` type
/// - Implement `Clone` (for shared access)
///
/// # Example
///
/// ```rust,ignore
/// use risten::tower::ServiceHook;
///
/// // Wrap a tower service as a Hook
/// let hook = ServiceHook::new(my_tower_service);
///
/// // Use in a static chain
/// let chain = static_hooks![hook, OtherHook];
/// ```
pub struct ServiceHook<S> {
    service: S,
}

impl<S> ServiceHook<S> {
    /// Create a new `ServiceHook` wrapping the given service.
    pub fn new(service: S) -> Self {
        Self { service }
    }

    /// Get a reference to the inner service.
    pub fn inner(&self) -> &S {
        &self.service
    }
}

impl<S: Clone> Clone for ServiceHook<S> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
        }
    }
}

impl<S, E> Hook<E> for ServiceHook<S>
where
    E: Message + Clone,
    S: ::tower::Service<E, Response = HookResult, Error = BoxError> + Send + Sync + Clone + 'static,
    S::Future: Send,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        let mut service = self.service.clone();
        // Note: We don't check poll_ready here for simplicity.
        // In production, you might want to handle backpressure.
        service.call(event.clone()).await
    }
}

// ============================================================================
// Layer Integration
// ============================================================================

/// Applies a tower `Layer` to a risten `Hook`.
///
/// This allows using tower's middleware (timeouts, rate limiting, etc.)
/// with risten Hooks.
///
/// # Type Parameters
///
/// - `L`: The Layer type
/// - `H`: The inner Hook type
/// - `E`: The event type
///
/// # Example
///
/// ```rust,ignore
/// use risten::tower::TowerLayerHook;
/// use tower::timeout::TimeoutLayer;
/// use std::time::Duration;
///
/// let inner_hook = MyHandler;
/// let with_timeout = TowerLayerHook::new(
///     TimeoutLayer::new(Duration::from_secs(5)),
///     inner_hook,
/// );
/// ```
pub struct TowerLayerHook<L, H, E>
where
    L: ::tower::Layer<HookService<H, E>>,
{
    layered_service: L::Service,
    _marker: PhantomData<(H, E)>,
}

impl<L, H, E> TowerLayerHook<L, H, E>
where
    L: ::tower::Layer<HookService<H, E>>,
{
    /// Create a new `TowerLayerHook` by applying a layer to a hook.
    pub fn new(layer: L, hook: H) -> Self {
        let inner_service = HookService::new(hook);
        let layered_service = layer.layer(inner_service);
        Self {
            layered_service,
            _marker: PhantomData,
        }
    }
}

impl<L, H, E> Clone for TowerLayerHook<L, H, E>
where
    L: ::tower::Layer<HookService<H, E>>,
    L::Service: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layered_service: self.layered_service.clone(),
            _marker: PhantomData,
        }
    }
}

// Note: Implementing Hook for TowerLayerHook is complex because
// Layer::Service may have different Response/Error types.
// For now, we provide the struct; full Hook impl requires
// the layered service to have compatible types.

impl<L, H, E> Hook<E> for TowerLayerHook<L, H, E>
where
    E: Message + Clone,
    H: Hook<E> + 'static,
    L: ::tower::Layer<HookService<H, E>> + Send + Sync + 'static,
    L::Service: ::tower::Service<E, Response = HookResult, Error = BoxError> + Send + Sync + Clone,
    <L::Service as ::tower::Service<E>>::Future: Send,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        let mut service = self.layered_service.clone();
        service.call(event.clone()).await
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Convert a Hook into a tower Service.
pub fn into_service<H, E>(hook: H) -> HookService<H, E>
where
    H: Hook<E>,
    E: Message,
{
    HookService::new(hook)
}

/// Convert a tower Service into a Hook.
pub fn from_service<S, E>(service: S) -> ServiceHook<S>
where
    S: ::tower::Service<E, Response = HookResult, Error = BoxError>,
    E: Message,
{
    ServiceHook::new(service)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestEvent {
        value: i32,
    }

    struct PassThroughHook;

    impl Hook<TestEvent> for PassThroughHook {
        async fn on_event(&self, _event: &TestEvent) -> Result<HookResult, BoxError> {
            Ok(HookResult::Next)
        }
    }

    #[test]
    fn test_hook_service_creation() {
        let hook = PassThroughHook;
        let _service: HookService<_, TestEvent> = HookService::new(hook);
    }

    #[test]
    fn test_service_hook_creation() {
        // Create a simple service that always returns Next
        let hook = PassThroughHook;
        let service: HookService<_, TestEvent> = HookService::new(hook);
        let _hook = ServiceHook::new(service);
    }

    #[tokio::test]
    async fn test_hook_service_call() {
        use ::tower::Service;

        let hook = PassThroughHook;
        let mut service: HookService<_, TestEvent> = HookService::new(hook);

        let event = TestEvent { value: 42 };
        let result = service.call(event).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HookResult::Next);
    }

    #[tokio::test]
    async fn test_service_hook_call() {
        let hook = PassThroughHook;
        let service: HookService<_, TestEvent> = HookService::new(hook);
        let hook_from_service = ServiceHook::new(service);

        let event = TestEvent { value: 42 };
        let result = hook_from_service.on_event(&event).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), HookResult::Next);
    }
}
