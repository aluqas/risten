/// A factory trait for creating hooks.
///
/// Since `inventory` collects static items, and `Hook`s might need to be created/cloned,
/// or simply because `Box<dyn Hook>` isn't `Sync` enough for static context easily without `Lazy`,
/// we collect factories that can produce hooks.
pub trait HookFactory<E>: Send + Sync + 'static {
    // Must return DynHook because Hook is not object safe (has async fn)
    fn create(&self) -> Box<dyn crate::flow::hook::DynHook<E>>;
}

impl<E, F> HookFactory<E> for F
where
    F: Fn() -> Box<dyn crate::flow::hook::DynHook<E>> + Send + Sync + 'static,
{
    fn create(&self) -> Box<dyn crate::flow::hook::DynHook<E>> {
        (self)()
    }
}

/// Defines a global registry for a specific event type.
///
/// This macro creates the necessary static infrastructure to collect hooks
/// for the given event type distributed across the codebase using `inventory`.
///
/// # Example
/// ```rust,ignore
/// use sakuramiya_event::model::Message;
/// struct MyEvent;
/// impl Message for MyEvent {}
///
/// sakuramiya_event::define_global_registry!(MyEvent);
/// ```
#[macro_export]
macro_rules! define_global_registry {
    ($event_type:ty) => {
        // Collect references to HookFactories
        $crate::inventory::collect!(
            &'static dyn $crate::orchestrator::macros::HookFactory<$event_type>
        );

        /// Creates a new Registry populated with all distributed hooks found.
        pub fn global_registry() -> $crate::orchestrator::registry::Registry<$event_type> {
            let mut builder = $crate::orchestrator::registry::RegistryBuilder::new();
            for factory in $crate::inventory::iter::<
                &'static dyn $crate::orchestrator::macros::HookFactory<$event_type>,
            > {
                builder.register_mut(factory.create());
            }
            builder.build()
        }
    };
}

/// Registers a hook to the global registry defined for the event type.
///
/// The expression provided must evaluate to something that implements `Hook<EventType>`.
/// It is wrapped in a factory function, so it can be re-instantiated if needed (though
/// typically called once at startup).
///
/// # Example
/// ```rust,ignore
/// register_hook!(MyEvent, MyHook::new());
/// ```
#[macro_export]
macro_rules! register_hook {
    ($event_type:ty, $hook_expr:expr) => {
        $crate::inventory::submit! {
            &(|| -> Box<dyn $crate::flow::hook::DynHook<$event_type>> {
                Box::new($hook_expr)
            }) as &'static dyn $crate::orchestrator::macros::HookFactory<$event_type>
        }
    };
}
