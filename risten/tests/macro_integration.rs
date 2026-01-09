//! Integration tests for risten macros.
//!
//! These tests define the expected API surface. If tests fail,
//! the implementation should be fixed, not the tests.

#![cfg(feature = "macros")]

use risten::{BoxError, Handler, Hook, HookResult, Listener, Message, Pipeline};

// Test Event Type
#[derive(Clone, Debug)]
struct TestEvent {
    id: u64,
    content: String,
}

impl Message for TestEvent {}

// Test: #[risten::event] basic usage
/// The simplest possible hook - just returns Next.
#[risten::event]
async fn simple_hook(event: &TestEvent) -> Result<HookResult, BoxError> {
    let _ = event;
    Ok(HookResult::Next)
}

#[tokio::test]
async fn test_event_macro_basic() {
    // The macro should generate a struct named `simple_hook`
    let hook = simple_hook;

    // It should implement Hook<TestEvent>
    let event = TestEvent {
        id: 1,
        content: "hello".to_string(),
    };

    let result = hook.on_event(&event).await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookResult::Next));
}

// Test: #[risten::event] can access event data
#[risten::event]
async fn logging_hook(event: &TestEvent) -> Result<HookResult, BoxError> {
    // Hook can read from event
    if event.content.is_empty() {
        return Ok(HookResult::Stop);
    }
    Ok(HookResult::Next)
}

#[tokio::test]
async fn test_event_macro_accesses_data() {
    let hook = logging_hook;

    // Non-empty content -> Next
    let event1 = TestEvent {
        id: 1,
        content: "hello".to_string(),
    };
    assert!(matches!(
        hook.on_event(&event1).await.unwrap(),
        HookResult::Next
    ));

    // Empty content -> Stop
    let event2 = TestEvent {
        id: 2,
        content: "".to_string(),
    };
    assert!(matches!(
        hook.on_event(&event2).await.unwrap(),
        HookResult::Stop
    ));
}

// Test: #[risten::handler] basic usage
/// A simple handler that transforms input.
#[risten::handler]
async fn echo_handler(input: String) -> String {
    format!("Echo: {}", input)
}

#[tokio::test]
async fn test_handler_macro_basic() {
    // The macro should generate a struct named `echo_handler`
    let handler = echo_handler;

    // It should implement Handler<String>
    let result = handler.call("hello".to_string()).await;
    assert_eq!(result, "Echo: hello");
}

// Test: #[risten::handler] with complex types
#[derive(Clone, Debug)]
struct Request {
    method: String,
    path: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Response {
    status: u16,
    body: String,
}

impl Message for Request {}
impl Message for Response {}

#[risten::handler]
async fn api_handler(req: Request) -> Response {
    Response {
        status: 200,
        body: format!("{} {}", req.method, req.path),
    }
}

#[tokio::test]
async fn test_handler_macro_complex_types() {
    let handler = api_handler;

    let req = Request {
        method: "GET".to_string(),
        path: "/users".to_string(),
    };

    let resp = handler.call(req).await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, "GET /users");
}

// Test: Hook works with static_hooks! macro
#[risten::event]
async fn counter_hook(event: &TestEvent) -> Result<HookResult, BoxError> {
    // Just process and continue
    let _id = event.id;
    Ok(HookResult::Next)
}

#[tokio::test]
async fn test_event_with_static_hooks() {
    use risten::{StaticRouter, static_hooks};

    // static_hooks! should accept macro-generated hooks
    let chain = static_hooks![simple_hook, counter_hook];
    let router = StaticRouter::new(chain);

    let event = TestEvent {
        id: 42,
        content: "test".to_string(),
    };

    // Route should work
    let result = router.route(event).await;
    assert!(result.is_ok());
}

// Test: Handler works with Pipeline
struct ExtractContent;

impl Listener<TestEvent> for ExtractContent {
    type Output = String;

    async fn listen(&self, event: &TestEvent) -> Result<Option<String>, BoxError> {
        Ok(Some(event.content.clone()))
    }
}

#[tokio::test]
async fn test_handler_with_pipeline() {
    // Pipeline = Listener + Handler
    let listener = ExtractContent;
    let pipeline = listener.handler(echo_handler);

    // Pipeline implements Hook
    let event = TestEvent {
        id: 1,
        content: "world".to_string(),
    };

    let result = pipeline.on_event(&event).await;
    assert!(result.is_ok());
}

// Test: Generated structs have expected traits
#[risten::event]
async fn trait_test_hook(_event: &TestEvent) -> Result<HookResult, BoxError> {
    Ok(HookResult::Next)
}

#[test]
fn test_event_macro_derives() {
    // Should be Clone
    let h1 = trait_test_hook;
    let h2 = h1.clone();
    let _ = h2;

    // Should be Copy
    let h3 = trait_test_hook;
    let h4 = h3;
    let _ = (h3, h4);

    // Should be Debug
    let debug_str = format!("{:?}", trait_test_hook);
    assert!(debug_str.contains("trait_test_hook"));

    // Should be Default
    let h5 = <trait_test_hook as Default>::default();
    let _ = h5;
}

#[risten::handler]
async fn handler_trait_test(input: String) -> String {
    input
}

#[test]
fn test_handler_macro_derives() {
    // Should be Clone, Copy, Debug, Default
    let h1 = handler_trait_test;
    let h2 = h1.clone();
    let h3 = h1;
    let _ = (h1, h2, h3);

    let debug_str = format!("{:?}", handler_trait_test);
    assert!(debug_str.contains("handler_trait_test"));

    let _h4 = <handler_trait_test as Default>::default();
}

// Test: #[risten::event(priority = N)]
#[risten::event(priority = 100)]
async fn high_priority_hook(event: &TestEvent) -> Result<HookResult, risten::BoxError> {
    let _ = event;
    Ok(HookResult::Next)
}

#[risten::event(priority = -10)]
async fn low_priority_hook(event: &TestEvent) -> Result<HookResult, risten::BoxError> {
    let _ = event;
    Ok(HookResult::Next)
}

#[test]
fn test_event_priority_attribute() {
    // Priority should be accessible as a const
    assert_eq!(high_priority_hook::PRIORITY, 100);
    assert_eq!(low_priority_hook::PRIORITY, -10);
}

#[risten::event(name = "CustomNamedHook")]
async fn internal_hook_impl(event: &TestEvent) -> Result<HookResult, risten::BoxError> {
    let _ = event;
    Ok(HookResult::Next)
}

#[tokio::test]
async fn test_event_custom_name() {
    // The struct should be named CustomNamedHook, not internal_hook_impl
    let hook = CustomNamedHook;

    let event = TestEvent {
        id: 1,
        content: "test".to_string(),
    };

    let result = hook.on_event(&event).await;
    assert!(result.is_ok());
}

// Test: #[risten::handler(name = "...")]
#[risten::handler(name = "CustomHandler")]
async fn handler_internal_impl(input: String) -> String {
    format!("Custom: {}", input)
}

#[tokio::test]
async fn test_handler_custom_name() {
    // The struct should be named CustomHandler
    let handler = CustomHandler;

    let result = handler.call("test".to_string()).await;
    assert_eq!(result, "Custom: test");
}

#[risten::event(priority = 50, name = "PrioritizedHook")]
async fn combined_attrs_impl(event: &TestEvent) -> Result<HookResult, risten::BoxError> {
    let _ = event;
    Ok(HookResult::Stop)
}

#[test]
fn test_combined_attributes() {
    // Should have both priority and custom name
    assert_eq!(PrioritizedHook::PRIORITY, 50);

    let debug_str = format!("{:?}", PrioritizedHook);
    assert!(debug_str.contains("PrioritizedHook"));
}

#[derive(Clone, Debug)]
struct MessageEvent {
    content: String,
}
impl Message for MessageEvent {}

#[derive(Clone, Debug)]
struct ReadyEvent {
    session_id: u64,
}
impl Message for ReadyEvent {}

#[derive(Clone, Debug)]
struct ErrorEvent {
    code: u32,
}
impl Message for ErrorEvent {}

#[risten::dispatch]
#[derive(Clone, Debug)]
enum AppEvent {
    Message(MessageEvent),
    Ready(ReadyEvent),
    Error(ErrorEvent),
    Shutdown,
}

#[test]
fn test_dispatch_variant_name() {
    // dispatch_match and variant_name should be generated
    let msg = AppEvent::Message(MessageEvent {
        content: "hello".to_string(),
    });
    assert_eq!(msg.variant_name(), "Message");

    let ready = AppEvent::Ready(ReadyEvent { session_id: 123 });
    assert_eq!(ready.variant_name(), "Ready");

    let err = AppEvent::Error(ErrorEvent { code: 500 });
    assert_eq!(err.variant_name(), "Error");

    let shutdown = AppEvent::Shutdown;
    assert_eq!(shutdown.variant_name(), "Shutdown");
}

#[test]
fn test_dispatch_match() {
    // dispatch_match should return HookResult for each variant
    let msg = AppEvent::Message(MessageEvent {
        content: "test".to_string(),
    });
    let result = msg.dispatch_match();
    assert!(matches!(result, HookResult::Next));

    let shutdown = AppEvent::Shutdown;
    let result = shutdown.dispatch_match();
    assert!(matches!(result, HookResult::Next));
}

// Define hooks for each event type
#[risten::event]
async fn on_message(event: &MessageEvent) -> Result<HookResult, risten::BoxError> {
    if event.content.is_empty() {
        Ok(HookResult::Stop)
    } else {
        Ok(HookResult::Next)
    }
}

#[risten::event]
async fn on_ready(event: &ReadyEvent) -> Result<HookResult, risten::BoxError> {
    if event.session_id == 0 {
        Ok(HookResult::Stop)
    } else {
        Ok(HookResult::Next)
    }
}

// Define enum with static handler bindings via doc comments
#[risten::dispatch]
#[derive(Clone, Debug)]
enum StaticAppEvent {
    /// @handler(on_message)
    Message(MessageEvent),

    /// @handler(on_ready)
    Ready(ReadyEvent),

    // No handler - should return Next
    Shutdown,
}

#[tokio::test]
async fn test_static_dispatch_to_hooks() {
    // Test Message variant with handler
    let msg = StaticAppEvent::Message(MessageEvent {
        content: "hello".to_string(),
    });
    let result = msg.dispatch_to_hooks().await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookResult::Next));

    // Test Message with empty content (should Stop)
    let empty_msg = StaticAppEvent::Message(MessageEvent {
        content: "".to_string(),
    });
    let result = empty_msg.dispatch_to_hooks().await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookResult::Stop));

    // Test Ready variant with handler
    let ready = StaticAppEvent::Ready(ReadyEvent { session_id: 123 });
    let result = ready.dispatch_to_hooks().await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookResult::Next));

    // Test Shutdown (no handler) - should return Next
    let shutdown = StaticAppEvent::Shutdown;
    let result = shutdown.dispatch_to_hooks().await;
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), HookResult::Next));
}

#[test]
fn test_static_dispatch_still_has_variant_name() {
    // Static dispatch should still have all the other methods
    let msg = StaticAppEvent::Message(MessageEvent {
        content: "test".to_string(),
    });
    assert_eq!(msg.variant_name(), "Message");

    let shutdown = StaticAppEvent::Shutdown;
    assert_eq!(shutdown.variant_name(), "Shutdown");
}
