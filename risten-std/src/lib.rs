//! # risten-std
//!
//! Standard implementations for the Risten event processing framework.
//!
//! This crate provides:
//!
//! ## Routers
//!
//! - **Static routing**: [`StaticRouter`], [`StaticFanoutRouter`] - Zero-cost, compile-time optimized
//! - **Dispatch routing**: [`DispatchRouter`] - Inventory-based automatic collection
//! - **Dynamic routing**: [`Registry`] - Runtime registration
//!
//! ## Helpers
//!
//! - **Standard hooks**: Logging, Timeout
//! - **Standard listeners**: Filter, Map
//! - **Macros**: [`static_hooks!`], [`static_fanout!`]
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use risten_std::{StaticRouter, static_hooks};
//!
//! // Create a zero-cost router with static hooks
//! let router = StaticRouter::new(static_hooks![
//!     LoggingHook,
//!     my_handler,
//! ]);
//!
//! // Or use inventory-based collection
//! use risten_std::routing::DispatchRouter;
//! let router = DispatchRouter::<MyEvent>::new();
//! ```

#![deny(clippy::pub_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

// Re-export core traits
pub use risten_core;

// Modules
pub mod dynamic;
pub mod hooks;
pub mod listeners;
pub mod routing;
pub mod static_dispatch;
pub mod testing;

#[cfg(feature = "inventory")]
pub use inventory;
