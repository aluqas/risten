//! Error types for Risten.
//!
//! This module provides a structured error hierarchy using `thiserror`:
//!
//! - [`RistenError`] - Top-level error type for all Risten operations
//! - [`DispatchError`] - Errors during event dispatch
//! - [`HookError`] - Errors from individual hooks
//! - [`RouterError`] - Errors from routing operations

use std::time::Duration;
use thiserror::Error;

/// A boxed error type for dynamic error handling.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Top-level error type for all Risten operations.
#[derive(Error, Debug)]
pub enum RistenError {
    /// An error occurred during event dispatch.
    #[error("dispatch error: {0}")]
    Dispatch(#[from] DispatchError),

    /// An error occurred in a hook.
    #[error("hook error: {0}")]
    Hook(#[from] HookError),

    /// An error occurred during routing.
    #[error("routing error: {0}")]
    Router(#[from] RouterError),

    /// A custom error occurred.
    #[error(transparent)]
    Custom(BoxError),
}

/// Errors that can occur during event dispatch.
#[derive(Error, Debug)]
pub enum DispatchError {
    /// An error occurred in a listener.
    #[error("listener error")]
    Listener(#[source] BoxError),

    /// A hook signaled early stop.
    #[error("hook returned early stop")]
    EarlyStop,

    /// No handlers were registered for the event.
    #[error("no handlers registered for this event type")]
    NoHandlers,

    /// The dispatcher was shut down.
    #[error("dispatcher has been shut down")]
    Shutdown,
}

/// Errors that can occur in hooks.
#[derive(Error, Debug)]
pub enum HookError {
    /// The hook panicked during execution.
    #[error("hook panicked: {0}")]
    Panic(String),

    /// The hook timed out.
    #[error("hook timed out after {0:?}")]
    Timeout(Duration),

    /// The hook was cancelled.
    #[error("hook was cancelled")]
    Cancelled,

    /// A custom hook error.
    #[error(transparent)]
    Custom(BoxError),
}

/// Errors that can occur during routing.
#[derive(Error, Debug)]
pub enum RouterError {
    /// No route was found for the given key.
    #[error("no route found for key: {0}")]
    NotFound(String),

    /// Route already exists.
    #[error("route already exists for key: {0}")]
    AlreadyExists(String),

    /// Invalid route configuration.
    #[error("invalid route configuration: {0}")]
    InvalidConfig(String),
}

// Convenience conversions
impl From<BoxError> for RistenError {
    fn from(err: BoxError) -> Self {
        RistenError::Custom(err)
    }
}

impl From<BoxError> for HookError {
    fn from(err: BoxError) -> Self {
        HookError::Custom(err)
    }
}

impl From<BoxError> for DispatchError {
    fn from(err: BoxError) -> Self {
        DispatchError::Listener(err)
    }
}
