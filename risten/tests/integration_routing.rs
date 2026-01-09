use risten::{
    Dispatcher, Hook, HookProvider, HookResult, Message,
    delivery::SequentialDelivery,
    dynamic::{DynamicDispatcher, DynamicHook},
    routing::{HashMapRouterBuilder, RouteResult, Router, RouterBuilder},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

mod common;
use common::TestEvent;

// A simple hook provider that uses a map of IDs to hooks
struct MapHookProvider {
    hooks: HashMap<i32, DynamicHook<TestEvent>>,
    router: Box<dyn Router<String, i32> + Send + Sync>,
}

impl MapHookProvider {
    fn new(router: Box<dyn Router<String, i32> + Send + Sync>) -> Self {
        Self {
            hooks: HashMap::new(),
            router,
        }
    }

    fn register<H: Hook<TestEvent>>(&mut self, id: i32, hook: H) {
        self.hooks.insert(id, DynamicHook::new(hook));
    }
}

// Implement HookProvider for our custom provider
impl HookProvider<TestEvent> for MapHookProvider {
    fn resolve<'a>(
        &'a self,
        event: &TestEvent,
    ) -> Box<dyn Iterator<Item = &'a dyn risten::DynHook<TestEvent>> + Send + 'a>
    where
        TestEvent: 'a,
    {
        // 1. Route the event content
        match self.router.route(&event.content) {
            RouteResult::Matched(id) => {
                // 2. Lookup hook by ID
                if let Some(hook) = self.hooks.get(id) {
                    // hook is DynamicHook, which implements Hook, thus implements DynHook.
                    // We return an iterator yielding a reference to it.
                    Box::new(std::iter::once(hook as &dyn risten::DynHook<TestEvent>))
                } else {
                    Box::new(std::iter::empty())
                }
            }
            _ => Box::new(std::iter::empty()),
        }
    }
}

#[derive(Clone, Copy)]
struct FlagHook {
    flag: &'static str,
}

impl Hook<TestEvent> for FlagHook {
    async fn on_event(
        &self,
        _event: &TestEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        // In a real test we would mutate state, but here we just return OK.
        // We verify via the resolved list size in the test logic or use a side-effect hook.
        Ok(HookResult::Next)
    }
}

struct SideEffectHook {
    visited: Arc<Mutex<Vec<String>>>,
    name: String,
}

impl Hook<TestEvent> for SideEffectHook {
    async fn on_event(
        &self,
        _event: &TestEvent,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        self.visited.lock().unwrap().push(self.name.clone());
        Ok(HookResult::Next)
    }
}

#[tokio::test]
async fn test_routing_dispatch_flow() {
    // 1. Setup Router
    let mut builder = HashMapRouterBuilder::default();
    builder.insert("route/a".to_string(), 1).unwrap();
    builder.insert("route/b".to_string(), 2).unwrap();
    let router = builder.build().unwrap();

    // 2. Setup Provider
    let mut provider =
        MapHookProvider::new(Box::new(router) as Box<dyn Router<String, i32> + Send + Sync>);

    let visited = Arc::new(Mutex::new(Vec::new()));

    provider.register(
        1,
        SideEffectHook {
            visited: visited.clone(),
            name: "HookA".to_string(),
        },
    );
    provider.register(
        2,
        SideEffectHook {
            visited: visited.clone(),
            name: "HookB".to_string(),
        },
    );

    // 3. Setup Dispatcher
    let dispatcher = DynamicDispatcher::new(provider, SequentialDelivery);

    // 4. Dispatch Event A
    let event_a = TestEvent {
        content: "route/a".to_string(),
    };
    dispatcher.dispatch(event_a).await.unwrap();

    {
        let v = visited.lock().unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], "HookA");
    }

    // 5. Dispatch Event B
    let event_b = TestEvent {
        content: "route/b".to_string(),
    };
    dispatcher.dispatch(event_b).await.unwrap();

    {
        let v = visited.lock().unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1], "HookB");
    }

    // 6. Dispatch Unknown
    let event_c = TestEvent {
        content: "route/unknown".to_string(),
    };
    dispatcher.dispatch(event_c).await.unwrap();

    {
        let v = visited.lock().unwrap();
        assert_eq!(v.len(), 2); // No change
    }
}
