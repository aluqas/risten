//! Dynamic routing implementations.
//!
//! This module provides runtime-flexible routing mechanisms.
//! Use when hook composition is determined at runtime (plugins, config-driven).

use risten_core::{
    BoxError, RoutingError, DynHook, HookResult, Listener, Message, RouteResult, Router,
};

/// A dynamic router that uses runtime hook resolution.
///
/// This router resolves hooks at runtime using a provider, allowing for
/// dynamic hook composition based on event contents or external configuration.
pub struct DynamicRouter<P, S> {
    provider: P,
    _strategy: S,
}

impl<P, S> DynamicRouter<P, S> {
    /// Create a new dynamic router with the given provider and strategy.
    pub fn new(provider: P, strategy: S) -> Self {
        Self {
            provider,
            _strategy: strategy,
        }
    }
}

impl<E, P, S> Router<E> for DynamicRouter<P, S>
where
    E: Message + Sync + 'static,
    P: HookProvider<E>,
    S: Send + Sync,
{
    type Error = RoutingError;

    async fn route(&self, event: &E) -> Result<RouteResult, Self::Error> {
        let hooks = self.provider.resolve(event);
        let mut stopped = false;
        for hook in hooks {
            match hook.on_event_dyn(event).await {
                Ok(HookResult::Stop) => {
                    stopped = true;
                    break;
                }
                Ok(HookResult::Next) => continue,
                Err(e) => return Err(RoutingError::Listener(e)),
            }
        }
        Ok(RouteResult {
            stopped,
            executed_count: 0, // Dynamic router doesn't track count
        })
    }
}

// DynamicRouter as Listener (Native Integration)
//
// When a Router acts as a Listener, its routing result determines the output:
// - `stopped = true` (a hook consumed the event) → `None` (event handled, skip downstream)
// - `stopped = false` (event passed through) → `Some(event)` (continue pipeline)
impl<E, P, S> Listener<E> for DynamicRouter<P, S>
where
    E: Message + Sync + Clone + 'static,
    P: HookProvider<E> + 'static,
    S: Send + Sync + 'static,
{
    type Output = E;

    async fn listen(&self, event: &E) -> Result<Option<Self::Output>, BoxError> {
        let result = <Self as Router<E>>::route(self, event)
            .await
            .map_err(|e| Box::new(e) as BoxError)?;

        if result.stopped {
            Ok(None)
        } else {
            Ok(Some(event.clone()))
        }
    }
}

/// Provider trait for hook resolution at runtime.
///
/// Implementors of this trait can dynamically determine which hooks should
/// process a given event based on runtime conditions.
pub trait HookProvider<E: Message>: Send + Sync {
    /// Resolve hooks for the given event.
    ///
    /// Returns an iterator over hooks that should process this event.
    /// The iterator yields references to trait objects, allowing for dynamic dispatch.
    fn resolve<'a>(&'a self, event: &E) -> Box<dyn Iterator<Item = &'a dyn DynHook<E>> + Send + 'a>
    where
        E: 'a;
}

impl<E: Message> HookProvider<E> for crate::dynamic::Registry<E> {
    fn resolve<'a>(&'a self, _event: &E) -> Box<dyn Iterator<Item = &'a dyn DynHook<E>> + Send + 'a>
    where
        E: 'a,
    {
        Box::new(self.hooks().map(|h| h.as_ref() as &dyn DynHook<E>))
    }
}

// Type alias for backward compatibility
/// Alias for dynamic router (compatibility with SimpleDynamicDispatcher).
pub type SimpleDynamicDispatcher<P, S> = DynamicRouter<P, S>;
