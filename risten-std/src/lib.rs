//! # risten-std
//!
//! Standard implementations for the Risten event processing framework.
//!
//! This crate provides:
//! - **Static dispatch**: [`HCons`], [`HNil`], [`StaticDispatcher`], [`static_hooks!`] macro
//! - **Dynamic dispatch**: [`Registry`]
//! - **Standard hooks**: Logging, Timeout
//! - **Standard listeners**: Filter, Map

#![deny(clippy::pub_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

// Re-export core traits
pub use risten_core;

// Modules
pub mod dynamic;
pub mod hooks;
pub mod listeners;
pub mod static_dispatch;
