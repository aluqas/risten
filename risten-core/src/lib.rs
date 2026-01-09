//! # risten-core
//!
//! Core traits for the Risten event processing framework.
//!
//! This crate provides the fundamental abstractions:
//! - [`Message`] - Marker trait for event types
//! - [`Hook`] / [`DynHook`] - Event processing primitives (low-level)
//! - [`Listener`] - Event transformation and filtering (high-level)
//! - [`Router`] / [`DynRouter`] - Execution engines (collection of hooks)
//! - [`Handler`] - Context-aware endpoint handlers
//!
//! ## Error Types
//!
//! - [`RistenError`] - Top-level error type
//! - [`DispatchError`] - Routing-related errors
//! - [`HookError`] - Hook execution errors
//!
//! This crate has minimal dependencies and is designed to be imported by
//! plugins and extensions that don't need the full `risten-std` implementation.

#![deny(clippy::pub_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

mod context;
mod error;
mod handler;
mod hook;
mod listener;
mod message;
mod response;
mod router;

// Re-exports
pub use context::{ExtractError, ExtractHandler, FromEvent};
pub use error::{BoxError, DispatchError, HookError, RistenError};
pub use handler::{Handler, HandlerResult};
pub use hook::{DynHook, Hook, HookResult};
pub use listener::{Chain, Listener, Pipeline};
pub use message::Message;
pub use response::{IntoHookOutcome, IntoResponse};
pub use router::{DynRouter, Router, RouterHook};
