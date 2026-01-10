use risten::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// Define an event
#[derive(Clone, Debug, Message)]
struct DemoEvent {
    id: usize,
    payload: String,
}

// Global counter to verify execution
static COUNTER: AtomicUsize = AtomicUsize::new(0);

// Define a handler
#[on(DemoEvent)]
async fn handle_event(event: Event<DemoEvent>) {
    println!("Handling event: {:?}", event.0);
    COUNTER.fetch_add(1, Ordering::SeqCst);
}

#[subscribe(DemoEvent)]
async fn handle_event_logging(event: Event<DemoEvent>) {
    println!("LOG: Event {} received", event.0.id);
    COUNTER.fetch_add(10, Ordering::SeqCst);
}

// A Listener that uses DispatchRouter
struct DemoListener;

impl Listener<DemoEvent> for DemoListener {
    type Output = ();

    async fn listen(&self, event: &DemoEvent) -> Result<Option<Self::Output>, BoxError> {
        let router = DispatchRouter::<DemoEvent>::new();
        router.route(event).await?;
        Ok(Some(()))
    }
}

#[tokio::test]
async fn test_architecture_demo() {
    let event = DemoEvent {
        id: 1,
        payload: "Hello Risten".to_string(),
    };

    let listener = DemoListener;
    listener.listen(&event).await.expect("Listen failed");

    // Total should be 11
    assert_eq!(COUNTER.load(Ordering::SeqCst), 11);
}
