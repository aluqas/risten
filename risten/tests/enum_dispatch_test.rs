//! Tests for the enum_hook! macro (RFC 0004 - Enum Dispatch)

use risten::{Dispatcher, Hook, HookResult, StaticDispatcher, enum_hook, static_hooks};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

// Test event type
#[derive(Clone, Debug)]
struct TestEvent {
    value: i32,
}

// Individual hook implementations
struct AddHook {
    amount: i32,
    counter: Arc<AtomicUsize>,
}

impl Hook<TestEvent> for AddHook {
    async fn on_event(
        &self,
        event: &TestEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        self.counter
            .fetch_add((event.value + self.amount) as usize, Ordering::SeqCst);
        Ok(HookResult::Next)
    }
}

struct MultiplyHook {
    factor: i32,
    counter: Arc<AtomicUsize>,
}

impl Hook<TestEvent> for MultiplyHook {
    async fn on_event(
        &self,
        event: &TestEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        self.counter
            .fetch_add((event.value * self.factor) as usize, Ordering::SeqCst);
        Ok(HookResult::Next)
    }
}

struct StopHook;

impl Hook<TestEvent> for StopHook {
    async fn on_event(
        &self,
        _event: &TestEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Stop)
    }
}

// Use the enum_hook! macro to create a unified hook enum
enum_hook! {
    /// Combined hook enum for testing
    pub enum TestHooks<TestEvent> {
        Add(AddHook),
        Multiply(MultiplyHook),
        Stop(StopHook),
    }
}

#[tokio::test]
async fn test_enum_hook_dispatch() {
    let counter = Arc::new(AtomicUsize::new(0));

    // Create hooks via the enum
    let hook1 = TestHooks::Add(AddHook {
        amount: 5,
        counter: counter.clone(),
    });
    let hook2 = TestHooks::Multiply(MultiplyHook {
        factor: 2,
        counter: counter.clone(),
    });

    // Both hooks implement Hook<TestEvent>
    let event = TestEvent { value: 10 };

    // Dispatch through the enum (static dispatch, no vtable)
    hook1.on_event(&event).await.unwrap();
    hook2.on_event(&event).await.unwrap();

    // hook1: 10 + 5 = 15
    // hook2: 10 * 2 = 20
    // Total: 35
    assert_eq!(counter.load(Ordering::SeqCst), 35);
}

#[tokio::test]
async fn test_enum_hook_from_impl() {
    let counter = Arc::new(AtomicUsize::new(0));

    // Test From impl for ergonomic construction
    let hook: TestHooks = AddHook {
        amount: 3,
        counter: counter.clone(),
    }
    .into();

    let event = TestEvent { value: 7 };
    hook.on_event(&event).await.unwrap();

    // 7 + 3 = 10
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

#[tokio::test]
async fn test_enum_hook_with_static_dispatcher() {
    let counter = Arc::new(AtomicUsize::new(0));

    // Create enum hooks
    let hook1 = TestHooks::Add(AddHook {
        amount: 1,
        counter: counter.clone(),
    });
    let hook2 = TestHooks::Multiply(MultiplyHook {
        factor: 3,
        counter: counter.clone(),
    });

    // Use with static_hooks! macro
    let chain = static_hooks![hook1, hook2];
    let dispatcher = StaticDispatcher::new(chain);

    dispatcher.dispatch(TestEvent { value: 5 }).await.unwrap();

    // hook1: 5 + 1 = 6
    // hook2: 5 * 3 = 15
    // Total: 21
    assert_eq!(counter.load(Ordering::SeqCst), 21);
}

#[tokio::test]
async fn test_enum_hook_stop_propagation() {
    let counter = Arc::new(AtomicUsize::new(0));

    // Stop hook should prevent further processing
    let hook1 = TestHooks::Stop(StopHook);
    let hook2 = TestHooks::Add(AddHook {
        amount: 100,
        counter: counter.clone(),
    });

    let chain = static_hooks![hook1, hook2];
    let dispatcher = StaticDispatcher::new(chain);

    dispatcher.dispatch(TestEvent { value: 5 }).await.unwrap();

    // hook2 should NOT execute due to Stop
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}
