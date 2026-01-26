//! # risten-core
//!
//! Core traits for the Risten event processing framework.
//!
//! This crate has minimal dependencies and is designed to be imported by
//! plugins and extensions that don't need the full `risten-std` implementation.
//!
//! # Core Components
//!
//! Risten is built on a layered architecture where each component has a clear responsibility:
//!
//! ## [`Hook`] - Primitive Kernel
//!
//! The lowest-level entry point for event processing. Similar to JavaScript
//! event handlers in its simplicity: receives an event, returns `Next` or `Stop`.
//!
//! - **Atomic**: The indivisible unit of event processing
//! - **Universal**: All higher abstractions ultimately convert to Hooks for execution
//! - **Low-Level Access**: Plugins and middleware can target this layer directly
//!
//! ## [`Router`] - Event Distribution Engine
//!
//! The core abstraction for **"how events flow"**. A Router is responsible for:
//!
//! - **Condition Dispatch**: Route events based on type/value (static match)
//! - **Collection**: Aggregate handlers defined across the codebase (e.g., via `inventory`)
//! - **Execution**: Control how handlers run (sequential, parallel, all-at-once)
//!
//! **Important**: Routers do NOT know about Extractors. They simply call `handler.call(event)`.
//! Argument resolution is the Handler's internal concern.
//!
//! ## [`Listener`] - Domain Gateway
//!
//! Wraps Hook mechanics to provide rich features: type transformation, filtering,
//! and pipeline composition.
//!
//! - **Wrapper**: Uses Hook internally while adding gatekeeping and transformation
//! - **Semantics**: "Listen and decide" — interpretation over action
//! - **Pipeline**: Combinators like `filter`, `map`, `then` enable declarative pipelines
//!
//! ## [`Handler`] - Execution Container
//!
//! Wraps user-defined functions to inject framework-specific context (Extractors,
//! error handling, etc.). This is the terminal point of the pipeline.
//!
//! - **Wrapper**: Adds event-architecture features to plain functions
//! - **Extractor Integration**: Via `ExtractHandler`, arguments are automatically resolved
//! - **Terminal**: The endpoint where business logic executes
//!
//! # Error Types
//!
//! - [`RistenError`] - Top-level error type
//! - [`RoutingError`] - Routing-related errors
//! - [`HookError`] - Hook execution errors

#![deny(clippy::wildcard_imports)]
#![warn(missing_docs)]

mod borrowed;
mod context;
mod error;
mod handler;
mod hook;
mod listener;
mod message;
mod response;
mod router;
mod shared;

// Re-exports
pub use borrowed::{BorrowedChain, BorrowedListener, RawMessage};
pub use context::{
    AsyncFromEvent, BorrowedExtractHandler, Event, ExtractError, ExtractHandler, FromEvent,
    FromEventGat, RefEvent, SyncExtractHandler,
};

pub use error::{BoxError, HookError, RistenError, RoutingError};
pub use handler::{DynHandler, Handler, HandlerResult};
pub use hook::{DynHook, Hook, HookResult};
pub use listener::{
    BoxListener, Catch, Chain, DynListener, Filter, FilterMap, Listener, Map, Pipeline, Then,
};
pub use message::Message;
pub use response::{Continue, Handled, IntoHookOutcome, IntoResponse};
pub use router::{DynRouter, ExecutionStrategy, RouteResult, Router, RouterHook};
pub use shared::SharedEvent;
