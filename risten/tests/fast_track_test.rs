#[cfg(feature = "phf")]
#[cfg(test)]
mod tests {
    use phf::phf_map;
    use risten::{
        dispatcher::{HCons, HNil, StaticFanoutDispatcher},
        presets::FastRouter, // Using the preset alias
        routing::{PhfRouter, Router},
        traits::{Dispatcher, DynDispatcher, Hook, HookResult, Message},
    };

    // Derived only, blanket impl applies
    #[derive(Clone, Debug)]
    struct TestEvent;

    struct HookA;
    impl Hook<TestEvent> for HookA {
        async fn on_event(
            &self,
            _: &TestEvent,
        ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
            Ok(HookResult::Next)
        }
    }

    struct HookB;
    impl Hook<TestEvent> for HookB {
        async fn on_event(
            &self,
            _: &TestEvent,
        ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
            Ok(HookResult::Next)
        }
    }

    // Chain definitions
    type Chain1 = HCons<HookA, HNil>;
    type Chain2 = HCons<HookA, HCons<HookB, HNil>>;

    static DISPATCHER_1: StaticFanoutDispatcher<Chain1> = StaticFanoutDispatcher {
        chain: HCons {
            head: HookA,
            tail: HNil,
        },
    };

    static DISPATCHER_2: StaticFanoutDispatcher<Chain2> = StaticFanoutDispatcher {
        chain: HCons {
            head: HookA,
            tail: HCons {
                head: HookB,
                tail: HNil,
            },
        },
    };

    // The user just uses FastRouter<E> which expands to PhfRouter<&'static (dyn DynDispatcher<E> + Sync)>
    static ROUTER_MAP: FastRouter<TestEvent> = phf_map! {
        "/route/1" => &DISPATCHER_1,
        "/route/2" => &DISPATCHER_2,
    };

    #[tokio::test]
    async fn test_fast_track_preset_usage() {
        let router = PhfRouter::new(&ROUTER_MAP);

        // Dispatch dynamically
        let d1 = router.route("/route/1").matched().expect("Should match 1");
        d1.dispatch_dyn(TestEvent).await.unwrap();

        let d2 = router.route("/route/2").matched().expect("Should match 2");
        d2.dispatch_dyn(TestEvent).await.unwrap();
    }
}
