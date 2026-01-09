use risten::{Hook, HookResult, Listener};
use std::sync::{Arc, Mutex, atomic::AtomicUsize};

mod common;
use common::{
    CollectingHandler, CountingHook, FallibleHandler, OrderRecordingHook, PrefixListener, TestEvent,
};

#[tokio::test]
async fn test_pipeline_flow() {
    // 1. Setup components
    let received_data = Arc::new(Mutex::new(Vec::new()));
    let call_count = Arc::new(AtomicUsize::new(0));

    // Listener: extracting content starting with "cmd:"
    let listener = PrefixListener {
        prefix: "cmd:".to_string(),
    };

    // Handler: collecting the extracted data
    let handler = CollectingHandler {
        received: received_data.clone(),
    };

    // Pipeline: connects listener and handler
    // Pipeline implements Hook<TestEvent>
    let pipeline = listener.handler(handler);

    // Hook: counts invocations
    let hook = CountingHook {
        call_count: call_count.clone(),
        result: HookResult::Next,
        priority: 0,
    };

    // 2. Execute Event 1 (Match)
    let event1 = TestEvent {
        content: "cmd:hello".to_string(),
    };

    // Run pre-hook
    hook.on_event(&event1).await.unwrap();
    // Run pipeline
    let _: HookResult = pipeline.on_event(&event1).await.unwrap();

    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    {
        let data = received_data.lock().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0], "hello");
    }

    // 3. Execute Event 2 (No Match)
    let event2 = TestEvent {
        content: "ignore:me".to_string(),
    };

    hook.on_event(&event2).await.unwrap();
    let _: HookResult = pipeline.on_event(&event2).await.unwrap();

    assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    {
        let data = received_data.lock().unwrap();
        assert_eq!(data.len(), 1); // Should not increase
    }
}

#[tokio::test]
async fn test_pipeline_error_handling() {
    // Listener that always matches "fail:"
    let listener = PrefixListener {
        prefix: "fail:".to_string(),
    };

    // Handler that fails
    let handler = FallibleHandler { should_fail: true };

    let pipeline = listener.handler(handler);

    let event = TestEvent {
        content: "fail:now".to_string(),
    };

    let result: Result<HookResult, Box<dyn std::error::Error + Send + Sync>> =
        pipeline.on_event(&event).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "intentional failure");
}

#[tokio::test]
async fn test_hook_chain_order() {
    let order_log = Arc::new(Mutex::new(Vec::new()));

    let hook1 = OrderRecordingHook {
        id: 1,
        order: order_log.clone(),
    };
    let hook2 = OrderRecordingHook {
        id: 2,
        order: order_log.clone(),
    };
    let hook3 = OrderRecordingHook {
        id: 3,
        order: order_log.clone(),
    };

    let event = TestEvent {
        content: "test".to_string(),
    };

    // Manual chaining simulation
    hook1.on_event(&event).await.unwrap();
    hook2.on_event(&event).await.unwrap();
    hook3.on_event(&event).await.unwrap();

    let log = order_log.lock().unwrap();
    assert_eq!(*log, vec![1, 2, 3]);
}
