use risten::{
    Hook, HookResult,
    delivery::{DeliveryStrategy, SequentialDelivery},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::time::Duration;

mod common;
use common::{CountingHook, TestEvent};

#[tokio::test]
async fn test_sequential_delivery_basic() {
    let count = Arc::new(AtomicUsize::new(0));
    let hook1 = CountingHook {
        call_count: count.clone(),
        result: HookResult::Next,
        priority: 0,
    };
    let hook2 = CountingHook {
        call_count: count.clone(),
        result: HookResult::Next,
        priority: 0,
    };

    let hooks_refs: Vec<&dyn risten::DynHook<TestEvent>> = vec![&hook1, &hook2];
    let strategy = SequentialDelivery::default();

    let result = strategy
        .deliver(
            TestEvent {
                content: "test".into(),
            },
            hooks_refs.into_iter(),
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_sequential_delivery_stop() {
    let count = Arc::new(AtomicUsize::new(0));
    let hook1 = CountingHook {
        call_count: count.clone(),
        result: HookResult::Stop,
        priority: 0,
    };
    let hook2 = CountingHook {
        call_count: count.clone(),
        result: HookResult::Next,
        priority: 0,
    };

    let hooks_refs: Vec<&dyn risten::DynHook<TestEvent>> = vec![&hook1, &hook2];
    let strategy = SequentialDelivery::default();

    let result = strategy
        .deliver(
            TestEvent {
                content: "test".into(),
            },
            hooks_refs.into_iter(),
        )
        .await;

    // SequentialDelivery returns Ok(()) even if stopped?
    // In sequential.rs: Ok(HookResult::Stop) => break (loop breaks).
    // Then returns Ok(()).
    // So distinct Outcome is not returned in Result.
    assert!(result.is_ok());
    // Executed count should be 1 because first hook returned Stop.
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

/*
#[tokio::test]
async fn test_fanout_delivery_parallel() {
    let count = Arc::new(AtomicUsize::new(0));

    struct SlowHook {
        count: Arc<AtomicUsize>,
    }

    impl Hook<TestEvent> for SlowHook {
        async fn on_event(
            &self,
            _event: &TestEvent,
        ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(HookResult::Next)
        }
    }

    let hooks = vec![
        SlowHook {
            count: count.clone(),
        },
        SlowHook {
            count: count.clone(),
        },
        SlowHook {
            count: count.clone(),
        },
    ];
    // let strategy = FanoutDelivery::new();

    // let start = std::time::Instant::now();
    // let result = strategy
    //     .deliver(
    //         &TestEvent {
    //             content: "test".into(),
    //         },
    //         &hooks,
    //     )
    //     .await;
    // let elapsed = start.elapsed();

    // assert_eq!(result.outcome, DeliveryOutcome::Completed);
    // assert_eq!(count.load(Ordering::SeqCst), 3);

    // // 3 hooks * 50ms = 150ms sequential. Parallel should be close to 50ms.
    // // Allow some margin but ensure it's significantly faster than sequential.
    // assert!(
    //     elapsed < Duration::from_millis(100),
    //     "Execution took too long for parallel dispatch: {:?}",
    //     elapsed
    // );
}
*/
