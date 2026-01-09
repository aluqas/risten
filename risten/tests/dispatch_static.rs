use risten::{StaticRouter, static_hooks};
use std::sync::{Arc, Mutex};

mod common;
use common::{OrderRecordingHook, TestEvent};

#[tokio::test]
async fn test_static_router() {
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

    // Using the static_hooks! macro preserves order
    let chain = static_hooks![hook1, hook2, hook3];
    let router = StaticRouter::new(chain);

    router
        .route(&TestEvent {
            content: "test".to_string(),
        })
        .await
        .unwrap();

    let executed_order = order.lock().unwrap();
    assert_eq!(
        *executed_order,
        vec![1, 2, 3],
        "Static hooks should execute in declaration order"
    );
}
