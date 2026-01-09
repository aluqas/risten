//! # risten - Static-First Event Processing Framework
//!
//! `risten` is an event processing framework designed with a **static-first** philosophy.
//! Compile-time optimizations are the default path; dynamic routing is available as an
//! explicit escape hatch for runtime flexibility.
//!
//! ## Quick Start (Static Path - Recommended)
//!
//! ```rust,ignore
//! use risten::{static_hooks, StaticRouter, HCons, HNil, Hook, HookResult};
//!
//! // Define your hooks
//! struct MyHook;
//! impl Hook<MyEvent> for MyHook { ... }
//!
//! // Build a static chain (zero-cost at runtime)
//! type MyChain = HCons<MyHook, HNil>;
//! static ROUTER: StaticRouter<MyChain> = ...;
//! ```

#![deny(clippy::pub_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

// ============================================================================
// Core Traits & Types (from risten-core)
// ============================================================================
pub use risten_core::{
    // Context / Extraction
    AsyncFromEvent,
    ExtractError,
    ExtractHandler,
    FromEvent,
    // Error types
    BoxError,
    DispatchError,
    HookError,
    RistenError,
    // Handler
    Handler,
    HandlerResult,
    // Hook
    DynHook,
    Hook,
    HookResult,
    // Listener (with declarative pipeline methods)
    BoxListener,
    Catch,
    Chain,
    DynListener,
    Filter,
    FilterMap,
    Listener,
    Map,
    Pipeline,
    Then,
    // Message
    Message,
    // Response
    IntoHookOutcome,
    IntoResponse,
    // Router Traits
    DynRouter,
    Router,
    RouterHook,
};

// ============================================================================
// Standard Implementations (from risten-std)
// ============================================================================

// Static Routing
pub use risten_std::{
    static_dispatch::{
        HCons, HListLen, HNil, HookChain, StaticChainBuilder, StaticRouter,
        fanout::{FanoutChain, StaticFanoutRouter},
    },
    static_fanout, static_hooks,
};

// Dynamic Routing
pub use risten_std::dynamic::{Registry, RegistryBuilder};

/// Dynamic routing support module.
pub mod dynamic {
    pub use risten_std::dynamic::{Registry, RegistryBuilder};
}

/// Delivery strategies for event processing.
pub mod delivery {
    /// Sequential delivery strategy (processes hooks one by one).
    #[derive(Clone, Copy, Debug, Default)]
    pub struct SequentialDelivery;
}

/// Standard hook implementations.
pub mod hooks {
    #![allow(clippy::wildcard_imports)]
    pub use risten_std::hooks::*;
}

/// Standard listener implementations.
pub mod listeners {
    #![allow(clippy::wildcard_imports)]
    pub use risten_std::listeners::*;
}

/// Testing utilities.
pub mod testing {
    #![allow(clippy::wildcard_imports)]
    pub use risten_std::testing::*;
}

/// Prelude module - common imports for Risten.
///
/// # Usage
///
/// ```rust,ignore
/// use risten::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{
        // Core traits
        Handler, Hook, HookResult, Listener, Message, Router,
        // Listener combinators
        BoxListener, Catch, Chain, Filter, FilterMap, Map, Pipeline, Then,
        // Extraction
        AsyncFromEvent, FromEvent,
        // Errors
        BoxError, DispatchError, ExtractError,
        // Response
        IntoResponse,
    };
}

// ============================================================================
// Compatibility Aliases
// ============================================================================

/// Alias for dynamic router (compatibility with SimpleDynamicDispatcher).
pub type SimpleDynamicDispatcher<P, S> = DynamicRouter<P, S>;

/// Dynamic router implementation.
///
/// This router resolves hooks at runtime using a provider.
pub struct DynamicRouter<P, S> {
    provider: P,
    _strategy: S,
}

impl<P, S> DynamicRouter<P, S> {
    /// Create a new dynamic router.
    pub fn new(provider: P, strategy: S) -> Self {
        Self {
            provider,
            _strategy: strategy,
        }
    }
}

impl<E, P, S> Router<E> for DynamicRouter<P, S>
where
    E: Message + Sync,
    P: HookProvider<E>,
    S: Send + Sync,
{
    type Error = DispatchError;

    async fn route(&self, event: &E) -> Result<(), Self::Error> {
        let hooks = self.provider.resolve(event);
        for hook in hooks {
            match hook.on_event_dyn(event).await {
                Ok(HookResult::Stop) => break,
                Ok(HookResult::Next) => continue,
                Err(e) => return Err(DispatchError::Listener(e)),
            }
        }
        Ok(())
    }
}

// DynamicRouter as Listener (Native Integration)
impl<E, P, S> Listener<E> for DynamicRouter<P, S>
where
    E: Message + Sync + Clone,
    P: HookProvider<E> + 'static,
    S: Send + Sync + 'static,
{
    type Output = E;

    async fn listen(&self, event: &E) -> Result<Option<Self::Output>, BoxError> {
        // Execute the router (zero-copy routing)
        Router::route(self, event)
            .await
            .map_err(|e| Box::new(e) as BoxError)?;
        // Clone only when returning to pass ownership downstream
        Ok(Some(event.clone()))
    }
}

/// Provider trait for hook resolution.
pub trait HookProvider<E: Message>: Send + Sync {
    /// Resolve hooks for the given event.
    fn resolve<'a>(&'a self, event: &E) -> Box<dyn Iterator<Item = &'a dyn DynHook<E>> + Send + 'a>
    where
        E: 'a;
}

impl<E: Message> HookProvider<E> for Registry<E> {
    fn resolve<'a>(&'a self, _event: &E) -> Box<dyn Iterator<Item = &'a dyn DynHook<E>> + Send + 'a>
    where
        E: 'a,
    {
        Box::new(self.hooks().map(|h| h.as_ref() as &dyn DynHook<E>))
    }
}

// ============================================================================
// Macros
// ============================================================================
#[cfg(feature = "macros")]
pub use risten_macros::{Message, dispatch, event, handler, main};

// ============================================================================
// Integration
// ============================================================================
#[cfg(feature = "inventory")]
pub use inventory;
