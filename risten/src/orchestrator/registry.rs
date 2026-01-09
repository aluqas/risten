//! Registry module for hook management.
//!
//! This module provides a builder pattern for registering hooks
//! and a frozen registry for immutable, thread-safe dispatch.

use crate::{
    core::{message::Message, response::IntoHookOutcome},
    flow::{
        handler::{Handler, HandlerResult},
        hook::{DynHook, Hook},
        listener::{Listener, Pipeline},
    },
    orchestrator::traits::HookProvider,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// A handle for dynamically toggling hook enabled state at runtime.
#[derive(Debug, Clone)]
pub struct EnabledHandle(Arc<AtomicBool>);

impl EnabledHandle {
    /// Create a new enabled handle with the given initial state.
    pub fn new(enabled: bool) -> Self {
        Self(Arc::new(AtomicBool::new(enabled)))
    }

    /// Check if the hook is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    /// Enable the hook.
    pub fn enable(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Disable the hook.
    pub fn disable(&self) {
        self.0.store(false, Ordering::Release);
    }

    /// Toggle the hook's enabled state, returning the new state.
    pub fn toggle(&self) -> bool {
        // Use fetch_xor for atomic toggle
        !self.0.fetch_xor(true, Ordering::AcqRel)
    }

    /// Set the hook's enabled state.
    pub fn set(&self, enabled: bool) {
        self.0.store(enabled, Ordering::Release);
    }
}

impl Default for EnabledHandle {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Metadata for a registered hook.
#[derive(Debug, Clone)]
pub struct RegistrationMeta {
    /// Priority (lower = executed first). Default is 0.
    pub priority: i32,
    /// Optional group name for filtering.
    pub group: Option<&'static str>,
    /// Handle for runtime enabled/disabled state.
    enabled: EnabledHandle,
}

impl Default for RegistrationMeta {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistrationMeta {
    /// Create default enabled metadata.
    pub fn new() -> Self {
        Self {
            priority: 0,
            group: None,
            enabled: EnabledHandle::new(true),
        }
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set group.
    pub fn with_group(mut self, group: &'static str) -> Self {
        self.group = Some(group);
        self
    }

    /// Set initial enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = EnabledHandle::new(enabled);
        self
    }

    /// Get a handle for toggling enabled state at runtime.
    pub fn enabled_handle(&self) -> EnabledHandle {
        self.enabled.clone()
    }

    /// Check if the hook is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.is_enabled()
    }
}

/// A hook entry with associated metadata.
pub struct HookEntry<E: Message> {
    hook: Box<dyn DynHook<E>>,
    meta: RegistrationMeta,
}

impl<E: Message> HookEntry<E> {
    /// Create a new hook entry.
    pub fn new<H: Hook<E> + 'static>(hook: H, meta: RegistrationMeta) -> Self {
        Self {
            hook: Box::new(hook),
            meta,
        }
    }

    /// Get the hook reference.
    pub fn hook(&self) -> &dyn DynHook<E> {
        &*self.hook
    }

    /// Get the metadata.
    pub fn meta(&self) -> &RegistrationMeta {
        &self.meta
    }

    /// Check if this hook is enabled.
    pub fn is_enabled(&self) -> bool {
        self.meta.is_enabled()
    }

    /// Get a handle for toggling this hook's enabled state at runtime.
    pub fn enabled_handle(&self) -> EnabledHandle {
        self.meta.enabled_handle()
    }
}

// ============================================================================
// RegistryBuilder - for constructing registries
// ============================================================================

/// Builder for constructing a Registry.
///
/// Use this to register hooks, then call `.build()` to create
/// an immutable, thread-safe `Registry`.
///
/// # Example
/// ```ignore
/// let registry = RegistryBuilder::new()
///     .register(my_hook)
///     .register_with_priority(important_hook, -10)
///     .build();
/// ```
pub struct RegistryBuilder<E: Message> {
    entries: Vec<HookEntry<E>>,
}

impl<E: Message> RegistryBuilder<E> {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a hook with default metadata.
    pub fn register<H: Hook<E> + 'static>(mut self, hook: H) -> Self {
        self.register_mut(hook);
        self
    }

    /// Register a hook with default metadata (mutable version).
    pub fn register_mut<H: Hook<E> + 'static>(&mut self, hook: H) {
        self.register_with_meta_mut(hook, RegistrationMeta::new());
    }

    /// Register a hook with specified metadata.
    pub fn register_with_meta<H: Hook<E> + 'static>(
        mut self,
        hook: H,
        meta: RegistrationMeta,
    ) -> Self {
        self.register_with_meta_mut(hook, meta);
        self
    }

    /// Register a hook with specified metadata (mutable version).
    pub fn register_with_meta_mut<H: Hook<E> + 'static>(
        &mut self,
        hook: H,
        meta: RegistrationMeta,
    ) {
        self.entries.push(HookEntry::new(hook, meta));
    }

    /// Register a hook with priority.
    pub fn register_with_priority<H: Hook<E> + 'static>(self, hook: H, priority: i32) -> Self {
        self.register_with_meta(hook, RegistrationMeta::new().with_priority(priority))
    }

    /// Register a hook with group.
    pub fn register_with_group<H: Hook<E> + 'static>(self, hook: H, group: &'static str) -> Self {
        self.register_with_meta(hook, RegistrationMeta::new().with_group(group))
    }

    /// Convenience method to register a pipeline directly.
    pub fn register_pipeline<L, H>(self, pipeline: Pipeline<L, H>) -> Self
    where
        E: Message + Sync,
        L: Listener<E> + 'static,
        H: Handler<L::Output> + 'static,
        L::Output: Send + Sync,
        H::Output: HandlerResult + IntoHookOutcome,
    {
        self.register(pipeline)
    }

    /// Build the immutable Registry.
    ///
    /// This sorts entries by priority and returns a frozen registry
    /// that can be safely shared across threads.
    pub fn build(mut self) -> Registry<E> {
        // Sort by priority (lower = first)
        self.entries.sort_by_key(|e| e.meta.priority);
        Registry {
            entries: self.entries,
        }
    }

    /// Get the number of registered hooks.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the builder has no hooks.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<E: Message> Default for RegistryBuilder<E> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Registry - immutable, thread-safe hook storage
// ============================================================================

/// An immutable, thread-safe registry of hooks.
///
/// Created by calling `RegistryBuilder::build()`. The registry is sorted
/// by priority and provides `&self` iteration for use in dispatchers.
///
/// # Example
/// ```ignore
/// let registry = RegistryBuilder::new()
///     .register(hook1)
///     .register(hook2)
///     .build();
///
/// // Can be shared via Arc
/// let shared = Arc::new(registry);
/// ```
pub struct Registry<E: Message> {
    entries: Vec<HookEntry<E>>,
}

impl<E: Message> Registry<E> {
    /// Iterate over all enabled hooks in priority order.
    ///
    /// This is the primary method for dispatchers to use.
    pub fn iter(&self) -> impl Iterator<Item = &dyn DynHook<E>> {
        self.entries
            .iter()
            .filter(|e| e.is_enabled())
            .map(|e| e.hook())
    }

    /// Iterate over hooks in a specific group.
    pub fn iter_group<'a>(
        &'a self,
        group: &'a str,
    ) -> impl Iterator<Item = &'a dyn DynHook<E>> + 'a {
        self.entries
            .iter()
            .filter(move |e| e.is_enabled() && e.meta.group == Some(group))
            .map(|e| e.hook())
    }

    /// Get the number of registered hooks.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries (for advanced use).
    pub fn entries(&self) -> &[HookEntry<E>] {
        &self.entries
    }
}

impl<E: Message> HookProvider<E> for Registry<E> {
    fn resolve<'a>(&'a self, _event: &E) -> Box<dyn Iterator<Item = &'a dyn DynHook<E>> + Send + 'a>
    where
        E: 'a,
    {
        Box::new(self.iter())
    }
}
