//! Dynamic registry for runtime hook registration.

use risten_core::{BoxError, DynHook, HookResult, Message};
use std::sync::Arc;

/// A registry of dynamically registered hooks.
pub struct Registry<E: Message> {
    hooks: Vec<Arc<dyn DynHook<E>>>,
}

impl<E: Message> Registry<E> {
    /// Dispatch an event to all registered hooks sequentially.
    pub async fn dispatch(&self, event: &E) -> Result<HookResult, BoxError> {
        for hook in &self.hooks {
            match hook.on_event_dyn(event).await? {
                HookResult::Stop => return Ok(HookResult::Stop),
                HookResult::Next => continue,
            }
        }
        Ok(HookResult::Next)
    }
}

/// Builder for constructing a Registry.
pub struct RegistryBuilder<E: Message> {
    hooks: Vec<Arc<dyn DynHook<E>>>,
}

impl<E: Message> Default for RegistryBuilder<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Message> RegistryBuilder<E> {
    /// Create a new empty registry builder.
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Register a hook.
    pub fn register<H: DynHook<E>>(mut self, hook: H) -> Self {
        self.hooks.push(Arc::new(hook));
        self
    }

    /// Build the registry.
    pub fn build(self) -> Registry<E> {
        Registry { hooks: self.hooks }
    }
}
