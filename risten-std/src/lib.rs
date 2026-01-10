//! # risten-std
//!
//! Standard implementations for the Risten event processing framework.
//!
//! This crate provides:
//! - **Static routing**: [`HCons`], [`HNil`], [`StaticRouter`], [`static_hooks!`] macro
//! - **Dynamic routing**: [`Registry`]
//! - **Standard hooks**: Logging, Timeout
//! - **Standard listeners**: Filter, Map
//! - **Dispatch routing**: [`DispatchRouter`]

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
