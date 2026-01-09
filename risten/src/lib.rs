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
pub use risten_std::static_dispatch::{
    HCons, HListLen, HNil, HookChain, StaticChainBuilder, StaticDispatcher,
    fanout::{FanoutChain, StaticFanoutDispatcher},
};
pub use risten_std::static_fanout;
pub use risten_std::static_hooks;

// Dynamic Dispatch
pub use risten_std::dynamic::{
    Registry, RegistryBuilder,
    routing::{HashMapRouter, HashMapRouterBuilder},
};

// Standard Components Modules
pub mod hooks {
    pub use risten_std::hooks::*;
}

pub mod listeners {
    pub use risten_std::listeners::*;
}

pub mod routing {
    pub use risten_std::routing::*;
}

// ============================================================================
// Macros
// ============================================================================
#[cfg(feature = "macros")]
pub use risten_macros::{dispatch, event, main};

// ============================================================================
// Integration
// ============================================================================
#[cfg(feature = "inventory")]
pub use inventory;
