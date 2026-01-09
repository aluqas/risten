use risten::{
    Dispatcher, HookResult, Listener, SimpleDynamicDispatcher,
    delivery::SequentialDelivery,
    dynamic::{EnabledHandle, RegistrationMeta, RegistryBuilder},
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
async fn test_pipeline_dispatch() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let handler = CollectingHandler {
        received: received.clone(),
    };

    let listener = PrefixListener {
        prefix: "!".to_string(),
    };

    let pipeline = listener.handler(handler);
    let registry = RegistryBuilder::new().register_pipeline(pipeline).build();
    let dispatcher = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());

    dispatcher
        .dispatch(TestEvent {
            content: "!hello".to_string(),
        })
        .await
        .unwrap();
    dispatcher
        .dispatch(TestEvent {
            content: "ignore".to_string(),
        })
        .await
        .unwrap();
    dispatcher
        .dispatch(TestEvent {
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
async fn test_priority_ordering() {
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

    // Register with different priorities (lower = first)
    let registry = RegistryBuilder::new()
        .register_with_priority(hook2, 10) // executed second
        .register_with_priority(hook3, 20) // executed third
        .register_with_priority(hook1, 5) // executed first
        .build();

    let dispatcher = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());
    dispatcher
        .dispatch(TestEvent {
            content: "test".to_string(),
        })
        .await
        .unwrap();

    let executed_order = order.lock().unwrap();
    assert_eq!(
        *executed_order,
        vec![1, 2, 3],
        "Hooks should execute in priority order"
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
        .register_with_priority(hook1, 0)
        .register_with_priority(hook2, 10)
        .build();

    let dispatcher = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());
    dispatcher
        .dispatch(TestEvent {
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
async fn test_enabled_handle_toggle() {
    let count = Arc::new(AtomicUsize::new(0));

    let hook = CountingHook {
        call_count: count.clone(),
        result: HookResult::Next,
        priority: 0,
    };

    let meta = RegistrationMeta::new();
    let handle = meta.enabled_handle();

    let registry = RegistryBuilder::new()
        .register_with_meta(hook, meta)
        .build();

    let dispatcher = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());

    // Initially enabled
    dispatcher
        .dispatch(TestEvent {
            content: "test1".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);

    // Disable the hook
    handle.disable();
    dispatcher
        .dispatch(TestEvent {
            content: "test2".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "Should not increment when disabled"
    );

    // Re-enable the hook
    handle.enable();
    dispatcher
        .dispatch(TestEvent {
            content: "test3".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_enabled_handle_toggle_method() {
    let handle = EnabledHandle::new(true);

    assert!(handle.is_enabled());

    let new_state = handle.toggle();
    assert!(!new_state);
    assert!(!handle.is_enabled());

    let new_state = handle.toggle();
    assert!(new_state);
    assert!(handle.is_enabled());
}

#[tokio::test]
async fn test_handler_error_propagation() {
    let listener = PrefixListener {
        prefix: "!".to_string(),
    };
    let handler = FallibleHandler { should_fail: true };

    let pipeline = listener.handler(handler);
    let registry = RegistryBuilder::new().register_pipeline(pipeline).build();
    let dispatcher = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());

    let result = dispatcher
        .dispatch(TestEvent {
            content: "!fail".to_string(),
        })
        .await;
    assert!(
        result.is_err(),
        "Dispatch should return error when handler fails"
    );
}

#[tokio::test]
async fn test_handler_success() {
    let listener = PrefixListener {
        prefix: "!".to_string(),
    };
    let handler = FallibleHandler { should_fail: false };

    let pipeline = listener.handler(handler);
    let registry = RegistryBuilder::new().register_pipeline(pipeline).build();
    let dispatcher = SimpleDynamicDispatcher::new(registry, SequentialDelivery::default());

    let result = dispatcher
        .dispatch(TestEvent {
            content: "!success".to_string(),
        })
        .await;
    assert!(
        result.is_ok(),
        "Dispatch should succeed when handler succeeds"
    );
}

#[tokio::test]
async fn test_group_iteration() {
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

    let registry = RegistryBuilder::new()
        .register_with_group(hook1, "group_a")
        .register_with_group(hook2, "group_b")
        .register_with_group(hook3, "group_a")
        .build();

    // Count hooks in group_a
    let group_a_count = registry.iter_group("group_a").count();
    assert_eq!(group_a_count, 2, "group_a should have 2 hooks");

    let group_b_count = registry.iter_group("group_b").count();
    assert_eq!(group_b_count, 1, "group_b should have 1 hook");
}
