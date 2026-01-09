use risten::{Handler, Hook, HookResult, Listener, Message};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

// ============================================================================
// Test Event Types
// ============================================================================

#[derive(Clone, Debug)]
pub struct TestEvent {
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct Trigger {
    pub data: String,
}

// ============================================================================
// Test Hooks and Handlers
// ============================================================================

pub struct CountingHook {
    pub call_count: Arc<AtomicUsize>,
    pub result: HookResult,
    pub priority: i32,
}

impl Hook<TestEvent> for CountingHook {
    async fn on_event(
        &self,
        _event: &TestEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(self.result)
    }
}

pub struct OrderRecordingHook {
    pub id: usize,
    pub order: Arc<Mutex<Vec<usize>>>,
}

impl Hook<TestEvent> for OrderRecordingHook {
    async fn on_event(
        &self,
        _event: &TestEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        self.order.lock().unwrap().push(self.id);
        Ok(HookResult::Next)
    }
}

pub struct PrefixListener {
    pub prefix: String,
}

impl Listener<TestEvent> for PrefixListener {
    type Output = Trigger;

    fn listen(&self, event: &TestEvent) -> Option<Self::Output> {
        if event.content.starts_with(&self.prefix) {
            Some(Trigger {
                data: event.content[self.prefix.len()..].to_string(),
            })
        } else {
            None
        }
    }
}

pub struct CollectingHandler {
    pub received: Arc<Mutex<Vec<String>>>,
}

impl Handler<Trigger> for CollectingHandler {
    type Output = ();

    async fn call(&self, input: Trigger) -> () {
        self.received.lock().unwrap().push(input.data);
    }
}

// Handler that returns Result for error handling test
pub struct FallibleHandler {
    pub should_fail: bool,
}

impl Handler<Trigger> for FallibleHandler {
    type Output = Result<(), std::io::Error>;

    async fn call(&self, _input: Trigger) -> Self::Output {
        if self.should_fail {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "intentional failure",
            ))
        } else {
            Ok(())
        }
    }
}
