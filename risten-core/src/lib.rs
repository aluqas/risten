//! # risten-core
//!
//! Core traits for the Risten event processing framework.
//!
//! This crate has minimal dependencies and is designed to be imported by
//! plugins and extensions that don't need the full `risten-std` implementation.
//!
//! # Four-Layer Architecture
//!
//! Risten is built on a strict 4-layer architecture, each layer serving a
//! distinct purpose in the event processing pipeline:
//!
//! ## Layer 1: Primitive Kernel ([`Hook`])
//!
//! The lowest-level entry point for ecosystem extensions. Similar to JavaScript
//! event handlers in its simplicity: receives an event, returns `Next` or `Stop`.
//!
//! - **Atomic**: The indivisible unit of event processing
//! - **Universal**: All higher abstractions (Listener, Router, Handler) ultimately
//!   convert to Hooks for execution
//! - **Low-Level Access**: Plugins and middleware can target this layer to operate
//!   independently of framework conveniences
//!
//! ## Layer 2: Rich Abstraction ([`Listener`])
//!
//! Wraps Hook mechanics to provide rich Listener Architecture features:
//! type transformation, filtering, and pipeline composition.
//!
//! - **Wrapper**: Uses Hook internally while adding gatekeeping and transformation
//! - **Semantics**: Not just "do" but "listen and decide" — interpretation over action
//! - **Pipeline**: Combinators like `filter`, `map`, `then` enable declarative pipelines
//!
//! ## Layer 3: Routing ([`Router`])
//!
//! An abstraction over Listeners that routes events to the next handler.
//! From outside, it's just another processing step; internally it manages
//! complex routing decisions.
//!
//! - **Abstraction**: Bundles multiple Listeners/Hooks into a single unit
//! - **Transparent**: Acts as a pass-through that dispatches to internal handlers
//! - **Composable**: Via [`RouterHook`], routers become Hooks themselves
//!
//! ## Layer 4: Context ([`Handler`])
//!
//! Wraps user-defined methods to inject framework-specific context (extractors,
//! error handling, etc.). This is the terminal point of the pipeline.
//!
//! - **Wrapper**: Adds event-architecture features to plain functions
//! - **Terminal**: The endpoint where business logic executes
//! - **Optional**: Users can implement Handler directly or use raw functions
//!
//! # Error Types
//!
//! - [`RistenError`] - Top-level error type
//! - [`DispatchError`] - Routing-related errors
//! - [`HookError`] - Hook execution errors

#![deny(clippy::wildcard_imports)]
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
pub use context::{
    AsyncFromEvent, Event, ExtractError, ExtractHandler, FromEvent, SyncExtractHandler,
};
pub use error::{BoxError, HookError, RistenError, RoutingError};
pub use handler::{Handler, HandlerResult};
pub use hook::{DynHook, Hook, HookResult};
pub use listener::{
    BoxListener, Catch, Chain, DynListener, Filter, FilterMap, Listener, Map, Pipeline, Then,
};
pub use message::Message;
pub use response::{Continue, Handled, IntoHookOutcome, IntoResponse};
pub use router::{DynRouter, RouteResult, Router, RouterHook};
