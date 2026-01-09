//! # risten-core
//!
//! Core traits for the Risten event processing framework.
//!
//! This crate provides the fundamental abstractions:
//! - [`Message`] - Marker trait for event types
//! - [`Hook`] / [`DynHook`] - Event processing primitives
//! - [`Handler`] - Endpoint handlers
//! - [`Listener`] - Event transformation and filtering
//! - [`Router`] - Key-based routing abstraction
//!
//! This crate has minimal dependencies and is designed to be imported by
//! plugins and extensions that don't need the full `risten-std` implementation.

#![deny(clippy::pub_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

mod borrowed;
mod context;
mod dispatcher;
mod error;
mod handler;
mod hook;
mod listener;
mod message;
mod response;
mod router;

// Re-exports
pub use borrowed::{BorrowedChain, BorrowedListener, RawMessage};
pub use context::{ExtractError, ExtractHandler, FromEvent};
pub use dispatcher::{Dispatcher, DynDispatcher};
pub use error::{BoxError, DispatchError};
pub use handler::{Handler, HandlerResult};
pub use hook::{DynHook, Hook, HookResult};
pub use listener::{Chain, Listener, Pipeline};
pub use message::Message;
pub use response::{IntoHookOutcome, IntoResponse};
pub use router::{RouteResult, Router, RouterBuildError, RouterBuilder};
