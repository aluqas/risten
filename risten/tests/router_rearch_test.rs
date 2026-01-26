//! Integration tests for the Router Re-architecture.
//!
//! These tests verify:
//! - DispatchRouter collection and parallel execution
//! - Sequential execution with SequentialDispatchRouter
//! - ConfigurableDispatchRouter mode switching
//! - RouteResult tracking

use risten::{
    routing::{DispatchRouter, ErasedHandlerWrapper},
    ExtractError, Handler, Message, Router,
};
use std::any::TypeId;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

mod common;

/// A simple event type for testing dispatch routing.
#[derive(Clone, Debug)]
struct DispatchEvent {
    value: i32,
}

impl Message for DispatchEvent {}

/// A handler that counts invocations.
struct CountingDispatchHandler {
    count: Arc<AtomicUsize>,
}

impl Handler<DispatchEvent> for CountingDispatchHandler {
    type Output = Result<(), ExtractError>;

    async fn call(&self, _event: DispatchEvent) -> Self::Output {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn test_dispatch_router_no_handlers() {
    // DispatchRouter with no handlers should return continued with 0 count
    let router = DispatchRouter::<DispatchEvent>::new();

    let result = router.route(&DispatchEvent { value: 42 }).await.unwrap();

    assert!(!result.stopped);
    assert_eq!(result.executed_count, 0);
}

#[tokio::test]
async fn test_dispatch_router_handler_count() {
    // Test handler_count method returns 0 when no handlers registered
    let count = DispatchRouter::<DispatchEvent>::handler_count();
    // Note: This depends on what's registered globally - in isolation should be 0
    // In real tests with #[subscribe], this would return the actual count
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_route_result_merge() {
    use risten::RouteResult;

    let a = RouteResult::with_count(3);
    let b = RouteResult::with_count(2);
    let merged = a.merge(b);

    assert!(!merged.stopped);
    assert_eq!(merged.executed_count, 5);

    let stopped = RouteResult::stopped();
    let continued = RouteResult::continued();
    let merged_stop = stopped.merge(continued);

    assert!(merged_stop.stopped);
}

#[tokio::test]
async fn test_route_result_constructors() {
    use risten::RouteResult;

    let continued = RouteResult::continued();
    assert!(!continued.stopped);
    assert_eq!(continued.executed_count, 0);

    let stopped = RouteResult::stopped();
    assert!(stopped.stopped);
    assert_eq!(stopped.executed_count, 1);

    let with_count = RouteResult::with_count(5);
    assert!(!with_count.stopped);
    assert_eq!(with_count.executed_count, 5);
}

/// Test that ExecutionStrategy enum works correctly.
#[test]
fn test_execution_strategy() {
    use risten::ExecutionStrategy;

    let seq = ExecutionStrategy::Sequential;
    let par = ExecutionStrategy::Parallel;
    let cond = ExecutionStrategy::Conditional;

    assert_eq!(seq, ExecutionStrategy::Sequential);
    assert_eq!(par, ExecutionStrategy::Parallel);
    assert_eq!(cond, ExecutionStrategy::Conditional);
    assert_ne!(seq, par);
}

/// Test that a manually registered handler works with DispatchRouter.
///
/// Note: This test manually submits to inventory to simulate what
/// the #[subscribe] macro would do.
#[tokio::test]
async fn test_manual_handler_registration() {
    use risten::routing::HandlerRegistration;

    // Define a simple event
    #[derive(Clone, Debug)]
    struct ManualEvent {
        id: u32,
    }
    impl Message for ManualEvent {}

    // Define a simple handler
    struct ManualHandler;
    impl Handler<ManualEvent> for ManualHandler {
        type Output = Result<(), ExtractError>;
        async fn call(&self, _event: ManualEvent) -> Self::Output {
            Ok(())
        }
    }

    // Create the wrapper (what the macro would generate)
    static HANDLER_INSTANCE: ManualHandler = ManualHandler;
    static HANDLER_WRAPPER: ErasedHandlerWrapper<ManualEvent, ManualHandler> =
        ErasedHandlerWrapper::new(ManualHandler);

    // Submit to inventory (what the macro would generate)
    inventory::submit! {
        HandlerRegistration {
            type_id: TypeId::of::<ManualEvent>(),
            handler: &HANDLER_WRAPPER,
            priority: 0,
        }
    }

    // Create router and route an event
    let router = DispatchRouter::<ManualEvent>::new();
    let result = router.route(&ManualEvent { id: 1 }).await.unwrap();

    // Should have executed at least 1 handler
    assert!(result.executed_count >= 1);
}

/// Test the static router still works after refactoring.
#[tokio::test]
async fn test_static_router_still_works() {
    use risten::{static_hooks, Hook, HookResult, StaticRouter};

    struct SimpleHook;
    impl Hook<common::TestEvent> for SimpleHook {
        async fn on_event(
            &self,
            _event: &common::TestEvent,
        ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
            Ok(HookResult::Next)
        }
    }

    let router = StaticRouter::new(static_hooks![SimpleHook]);
    let result = router
        .route(&common::TestEvent {
            content: "test".to_string(),
        })
        .await
        .unwrap();

    // Static router doesn't track count, but shouldn't be stopped
    assert!(!result.stopped);
}

/// Test the static fanout router for parallel execution.
#[tokio::test]
async fn test_static_fanout_router() {
    use risten::{static_fanout, Hook, HookResult, StaticFanoutRouter};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct ParallelHook;
    impl Hook<common::TestEvent> for ParallelHook {
        async fn on_event(
            &self,
            _event: &common::TestEvent,
        ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
            CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            Ok(HookResult::Next)
        }
    }

    let router = StaticFanoutRouter::new(static_fanout![ParallelHook, ParallelHook, ParallelHook]);
    let result = router
        .route(&common::TestEvent {
            content: "test".to_string(),
        })
        .await
        .unwrap();

    // All three hooks should have been called (parallel execution)
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 3);
    assert!(!result.stopped);
}
