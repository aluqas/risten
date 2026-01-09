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
    DispatchError,
    // Hook
    DynHook,
    DynListener,
    // Router Traits
    DynRouter,
    Event,
    ExtractError,
    ExtractHandler,
    Filter,
    FilterMap,
    FromEvent,
    Handled,
    // Handler
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

// Dynamic Routing
pub use risten_std::dynamic::{
    DynamicRouter, HookProvider, Registry, RegistryBuilder, SimpleDynamicDispatcher,
};

/// Dynamic routing support module.
pub mod dynamic {
    pub use risten_std::dynamic::{
        DynamicRouter, HookProvider, Registry, RegistryBuilder, SimpleDynamicDispatcher,
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
///
/// # Usage
///
/// ```rust,ignore
/// use risten::prelude::*;
/// ```
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
        DispatchError,
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
        Then,
    };
}

#[cfg(feature = "macros")]
pub use risten_macros::{Message, dispatch, event, handler, main};

#[cfg(feature = "inventory")]
pub use inventory;
