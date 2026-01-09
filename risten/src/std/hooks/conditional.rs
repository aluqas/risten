//! Conditional Hook - Execute hooks based on conditions.

use crate::{
    core::{error::BoxError, message::Message},
    flow::hook::{Hook, HookResult},
};

/// A Hook that conditionally executes an inner Hook.
///
/// The inner hook is only executed if the condition function returns `true`.
/// When the condition is `false`, the event is passed through unchanged.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{ConditionalHook, static_hooks};
///
/// // Only process events from admins
/// let admin_only = ConditionalHook::new(
///     |event: &UserEvent| event.is_admin,
///     AdminHandler,
/// );
///
/// let chain = static_hooks![LoggingHook::new(), admin_only, FallbackHandler];
/// ```
pub struct ConditionalHook<C, H> {
    condition: C,
    inner: H,
}

impl<C, H> ConditionalHook<C, H> {
    /// Create a new `ConditionalHook`.
    ///
    /// The inner hook will only be executed when `condition(event)` returns `true`.
    pub fn new(condition: C, inner: H) -> Self {
        Self { condition, inner }
    }
}

impl<E, C, H> Hook<E> for ConditionalHook<C, H>
where
    E: Message + Sync,
    C: Fn(&E) -> bool + Send + Sync + 'static,
    H: Hook<E>,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        if (self.condition)(event) {
            self.inner.on_event(event).await
        } else {
            Ok(HookResult::Next)
        }
    }
}

/// A Hook that executes one of two inner hooks based on a condition.
///
/// When the condition is `true`, the `then_hook` is executed.
/// When the condition is `false`, the `else_hook` is executed.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{BranchHook, static_hooks};
///
/// // Route to different handlers based on event type
/// let branch = BranchHook::new(
///     |e: &Event| e.is_priority,
///     PriorityHandler,
///     NormalHandler,
/// );
/// ```
pub struct BranchHook<C, T, E> {
    condition: C,
    then_hook: T,
    else_hook: E,
}

impl<C, T, E> BranchHook<C, T, E> {
    /// Create a new `BranchHook`.
    pub fn new(condition: C, then_hook: T, else_hook: E) -> Self {
        Self {
            condition,
            then_hook,
            else_hook,
        }
    }
}

impl<Ev, C, T, E> Hook<Ev> for BranchHook<C, T, E>
where
    Ev: Message + Sync,
    C: Fn(&Ev) -> bool + Send + Sync + 'static,
    T: Hook<Ev>,
    E: Hook<Ev>,
{
    async fn on_event(&self, event: &Ev) -> Result<HookResult, BoxError> {
        if (self.condition)(event) {
            self.then_hook.on_event(event).await
        } else {
            self.else_hook.on_event(event).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone, Debug)]
    struct TestEvent {
        value: i32,
    }

    struct CountingHook {
        count: Arc<AtomicUsize>,
    }

    impl Hook<TestEvent> for CountingHook {
        async fn on_event(&self, _event: &TestEvent) -> Result<HookResult, BoxError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(HookResult::Next)
        }
    }

    #[tokio::test]
    async fn test_conditional_hook_true() {
        let count = Arc::new(AtomicUsize::new(0));
        let hook = ConditionalHook::new(
            |e: &TestEvent| e.value > 5,
            CountingHook {
                count: Arc::clone(&count),
            },
        );

        let event = TestEvent { value: 10 };
        hook.on_event(&event).await.unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_conditional_hook_false() {
        let count = Arc::new(AtomicUsize::new(0));
        let hook = ConditionalHook::new(
            |e: &TestEvent| e.value > 5,
            CountingHook {
                count: Arc::clone(&count),
            },
        );

        let event = TestEvent { value: 3 };
        hook.on_event(&event).await.unwrap();

        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_branch_hook_then() {
        let then_count = Arc::new(AtomicUsize::new(0));
        let else_count = Arc::new(AtomicUsize::new(0));

        let hook = BranchHook::new(
            |e: &TestEvent| e.value > 5,
            CountingHook {
                count: Arc::clone(&then_count),
            },
            CountingHook {
                count: Arc::clone(&else_count),
            },
        );

        let event = TestEvent { value: 10 };
        hook.on_event(&event).await.unwrap();

        assert_eq!(then_count.load(Ordering::SeqCst), 1);
        assert_eq!(else_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_branch_hook_else() {
        let then_count = Arc::new(AtomicUsize::new(0));
        let else_count = Arc::new(AtomicUsize::new(0));

        let hook = BranchHook::new(
            |e: &TestEvent| e.value > 5,
            CountingHook {
                count: Arc::clone(&then_count),
            },
            CountingHook {
                count: Arc::clone(&else_count),
            },
        );

        let event = TestEvent { value: 3 };
        hook.on_event(&event).await.unwrap();

        assert_eq!(then_count.load(Ordering::SeqCst), 0);
        assert_eq!(else_count.load(Ordering::SeqCst), 1);
    }
}
