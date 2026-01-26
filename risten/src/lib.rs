//! # risten - Static-First Event Processing Framework
//!
//! `risten` is an event processing framework designed with a **static-first** philosophy.
//! Compile-time optimizations are the default path; dynamic routing is available as an
//! explicit escape hatch for runtime flexibility.

#![deny(clippy::pub_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

pub use risten_core::{
    // Context / Extraction
    AsyncFromEvent,
    // Error types
    BoxError,
    // Listener (with declarative pipeline methods)
    BoxListener,
    Catch,
    Chain,
    // Response
    Continue,
    // Hook
    DynHook,
    DynListener,
    // Router Traits
    DynRouter,
    Event,
    // Execution Strategy
    ExecutionStrategy,
    ExtractError,
    ExtractHandler,
    Filter,
    FilterMap,
    FromEvent,
    Handled,
    // Handler
    DynHandler,
    Handler,
    HandlerResult,
    Hook,
    HookError,
    HookResult,
    IntoHookOutcome,
    IntoResponse,
    Listener,
    Map,
    // Message
    Message,
    Pipeline,
    RistenError,
    RouteResult,
    Router,
    RouterHook,
    RoutingError,
    SyncExtractHandler,
    Then,
};

// Static Routing
pub use risten_std::{
    static_dispatch::{
        HCons, HListLen, HNil, HookChain, StaticChainBuilder, StaticRouter,
        fanout::{FanoutChain, StaticFanoutRouter},
    },
    static_fanout, static_hooks,
};

// Dynamic Routing & New Dispatch
pub use risten_std::{
    dynamic::{
        DynamicRouter, HookProvider, Registry, RegistryBuilder, SimpleDynamicDispatcher,
    },
    routing::{
        dispatch::{DispatchRouter, HandlerRegistration, ErasedHandlerWrapper},
    }
};

/// Dynamic routing support module.
pub mod dynamic {
    pub use risten_std::dynamic::{
        DynamicRouter, HookProvider, Registry, RegistryBuilder, SimpleDynamicDispatcher,
    };
}

/// Routing components.
pub mod routing {
    pub use risten_std::routing::{
        dispatch::{DispatchRouter, HandlerRegistration, ErasedHandlerWrapper},
    };
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
pub mod prelude {
    pub use crate::{
        // Extraction
        AsyncFromEvent,
        // Errors
        BoxError,
        // Listener combinators
        BoxListener,
        Catch,
        Chain,
        ExtractError,
        Filter,
        FilterMap,
        FromEvent,
        // Core traits
        Handler,
        Hook,
        HookResult,
        // Response
        IntoResponse,
        Listener,
        Map,
        Message,
        Pipeline,
        Router,
        RoutingError,
        Then,
        // New Router
        DispatchRouter,
        // Event Wrapper
        Event,
        DynHandler,
    };

    #[cfg(feature = "macros")]
    pub use crate::{on, subscribe, handler};
}

#[cfg(feature = "macros")]
pub use risten_macros::{Message, dispatch, event, handler, main, subscribe, on};

#[cfg(feature = "inventory")]
pub use inventory;
