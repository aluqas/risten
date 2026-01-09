//! Simple dispatch tests using the available API.

use risten::{
    HookResult, Listener, Router, SimpleDynamicDispatcher,
    delivery::SequentialDelivery,
    dynamic::RegistryBuilder,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

mod common;
use common::{
    CollectingHandler, CountingHook, FallibleHandler, OrderRecordingHook, PrefixListener, TestEvent,
};

#[tokio::test]
async fn test_pipeline_route() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let handler = CollectingHandler {
        received: received.clone(),
    };

    let listener = PrefixListener {
        prefix: "!".to_string(),
    };

    let pipeline = listener.handler(handler);
    let registry = RegistryBuilder::new().register(pipeline).build();
    let router = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());

    router
        .route(TestEvent {
            content: "!hello".to_string(),
        })
        .await
        .unwrap();
    router
        .route(TestEvent {
            content: "ignore".to_string(),
        })
        .await
        .unwrap();
    router
        .route(TestEvent {
            content: "!world".to_string(),
        })
        .await
        .unwrap();

    let guard = received.lock().unwrap();
    assert_eq!(guard.len(), 2);
    assert_eq!(guard[0], "hello");
    assert_eq!(guard[1], "world");
}

#[tokio::test]
async fn test_hook_ordering() {
    let order = Arc::new(Mutex::new(Vec::new()));

    let hook1 = OrderRecordingHook {
        id: 1,
        order: order.clone(),
    };
    let hook2 = OrderRecordingHook {
        id: 2,
        order: order.clone(),
    };
    let hook3 = OrderRecordingHook {
        id: 3,
        order: order.clone(),
    };

    // Hooks are registered and executed in order
    let registry = RegistryBuilder::new()
        .register(hook1)
        .register(hook2)
        .register(hook3)
        .build();

    let router = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());
    router
        .route(TestEvent {
            content: "test".to_string(),
        })
        .await
        .unwrap();

    let executed_order = order.lock().unwrap();
    assert_eq!(
        *executed_order,
        vec![1, 2, 3],
        "Hooks should execute in registration order"
    );
}

#[tokio::test]
async fn test_stop_propagation() {
    let count1 = Arc::new(AtomicUsize::new(0));
    let count2 = Arc::new(AtomicUsize::new(0));

    let hook1 = CountingHook {
        call_count: count1.clone(),
        result: HookResult::Stop, // This should stop propagation
        priority: 0,
    };
    let hook2 = CountingHook {
        call_count: count2.clone(),
        result: HookResult::Next,
        priority: 10,
    };

    let registry = RegistryBuilder::new()
        .register(hook1)
        .register(hook2)
        .build();

    let router = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());
    router
        .route(TestEvent {
            content: "test".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(
        count1.load(Ordering::SeqCst),
        1,
        "First hook should be called"
    );
    assert_eq!(
        count2.load(Ordering::SeqCst),
        0,
        "Second hook should NOT be called (stopped)"
    );
}

#[tokio::test]
async fn test_handler_error_propagation() {
    let listener = PrefixListener {
        prefix: "!".to_string(),
    };
    let handler = FallibleHandler { should_fail: true };

    let pipeline = listener.handler(handler);
    let registry = RegistryBuilder::new().register(pipeline).build();
    let router = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());

    let result = router
        .route(TestEvent {
            content: "!fail".to_string(),
        })
        .await;
    assert!(
        result.is_err(),
        "Route should return error when handler fails"
    );
}

#[tokio::test]
async fn test_handler_success() {
    let listener = PrefixListener {
        prefix: "!".to_string(),
    };
    let handler = FallibleHandler { should_fail: false };

    let pipeline = listener.handler(handler);
    let registry = RegistryBuilder::new().register(pipeline).build();
    let router = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());

    let result = router
        .route(TestEvent {
            content: "!success".to_string(),
        })
        .await;
    assert!(
        result.is_ok(),
        "Route should succeed when handler succeeds"
    );
}
