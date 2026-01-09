//! # risten - Static-First Event Processing Framework
//!
//! `risten` is an event processing framework designed with a **static-first** philosophy.
//! Compile-time optimizations are the default path; dynamic dispatch is available as an
//! explicit escape hatch for runtime flexibility.
//!
//! ## Quick Start (Static Path - Recommended)
//!
//! ```rust,ignore
//! use risten::{static_hooks, StaticDispatcher, HCons, HNil, Hook, HookResult};
//!
//! // Define your hooks
//! struct MyHook;
//! impl Hook<MyEvent> for MyHook { ... }
//!
//! // Build a static chain (zero-cost at runtime)
//! type MyChain = HCons<MyHook, HNil>;
//! static DISPATCHER: StaticDispatcher<MyChain> = ...;
//! ```

#![deny(clippy::pub_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

// ============================================================================
// Core Traits & Types (from risten-core)
// ============================================================================
pub use risten_core::{
    // Zero-Copy
    BorrowedChain,
    BorrowedListener,
    // Error
    BoxError,
    // Listener
    Chain,
    DispatchError,
    // Dispatcher Traits
    Dispatcher,
    DynDispatcher,
    // Hook
    DynHook,
    // Context / Extraction
    ExtractError,
    ExtractHandler,
    FromEvent,
    // Handler
    Handler,
    HandlerResult,
    Hook,
    HookResult,
    // Response
    IntoHookOutcome,
    IntoResponse,
    Listener,
    // Message
    Message,
    Pipeline,
    RawMessage,
    RouteResult,
    // Router
    Router,
    RouterBuildError,
    RouterBuilder,
};

// ============================================================================
// Standard Implementations (from risten-std)
// ============================================================================

// Static Dispatch
pub use risten_std::{
    static_dispatch::{
        HCons, HListLen, HNil, HookChain, StaticChainBuilder, StaticDispatcher,
        fanout::{FanoutChain, StaticFanoutDispatcher},
    },
    static_fanout, static_hooks,
};

// Dynamic Dispatch
pub use risten_std::dynamic::{
    Registry, RegistryBuilder,
    routing::{HashMapRouter, HashMapRouterBuilder},
};

/// Dynamic dispatch support module.
pub mod dynamic {
    pub use risten_std::dynamic::{
        Registry, RegistryBuilder,
        routing::{HashMapRouter, HashMapRouterBuilder},
    };
}

/// Delivery strategies for event processing.
pub mod delivery {
    // Re-export placeholder - to be implemented in risten-std
    /// Sequential delivery strategy (processes hooks one by one).
    #[derive(Clone, Copy, Debug, Default)]
    pub struct SequentialDelivery;
}

// Standard Components Modules
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

/// Routing implementations.
pub mod routing {
    #![allow(clippy::wildcard_imports)]
    pub use risten_std::routing::*;
    // Re-export core Router trait for compatibility
    pub use risten_core::{RouteResult, Router, RouterBuilder};
    // Re-export dynamic routing
    pub use risten_std::dynamic::routing::{HashMapRouter, HashMapRouterBuilder};
}

// ============================================================================
// Compatibility Aliases
// ============================================================================

/// Alias for dynamic dispatcher (compatibility).
pub type SimpleDynamicDispatcher<P, S> = DynamicDispatcher<P, S>;

/// Dynamic dispatcher implementation.
pub struct DynamicDispatcher<P, S> {
    provider: P,
    _strategy: S,
}

impl<P, S> DynamicDispatcher<P, S> {
    /// Create a new dynamic dispatcher.
    pub fn new(provider: P, strategy: S) -> Self {
        Self {
            provider,
            _strategy: strategy,
        }
    }
}

impl<E, P, S> Dispatcher<E> for DynamicDispatcher<P, S>
where
    E: Message + Clone + Sync,
    P: HookProvider<E>,
    S: Send + Sync,
{
    type Error = DispatchError;

    async fn dispatch(&self, event: E) -> Result<(), Self::Error> {
        let hooks = self.provider.resolve(&event);
        for hook in hooks {
            match hook.on_event_dyn(&event).await {
                Ok(HookResult::Stop) => break,
                Ok(HookResult::Next) => continue,
                Err(e) => return Err(DispatchError::ListenerError(e)),
            }
        }
        Ok(())
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
pub use risten_macros::{dispatch, event, handler, main};

// ============================================================================
// Integration
// ============================================================================
#[cfg(feature = "inventory")]
pub use inventory;
