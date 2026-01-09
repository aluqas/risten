#![cfg(feature = "macros")]

use risten::{BoxError, Hook, HookResult, Message};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

// ============================================================================
// Test: derive(Message)
// ============================================================================

#[derive(Clone, Debug, risten::Message)]
struct DerivedMessageEvent {
    content: String,
}

#[test]
fn test_derive_message() {
    fn assert_message<T: Message>() {}
    assert_message::<DerivedMessageEvent>();
}

// ============================================================================
// Test: #[event(filter = ...)]
// ============================================================================

// Global counter for testing
static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
struct MessageEvent {
    content: String,
}

impl Message for MessageEvent {}

#[risten::event(filter = |e: &MessageEvent| e.content.len() > 5)]
async fn on_long_message(event: &MessageEvent) -> Result<HookResult, BoxError> {
    CALL_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(HookResult::Next)
}

#[tokio::test]
async fn test_event_filter() {
    // Reset counter
    CALL_COUNT.store(0, Ordering::Relaxed);

    let hook = on_long_message;

    // Short message - should be filtered (no increment)
    let short_event = MessageEvent {
        content: "short".to_string(),
    };
    let result = hook.on_event(&short_event).await;
    assert!(result.is_ok());
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 0);

    // Long message - should pass filter (increment)
    let long_event = MessageEvent {
        content: "long message".to_string(),
    };
    let result = hook.on_event(&long_event).await;
    assert!(result.is_ok());
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 1);
}
