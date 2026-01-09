//! Error types for Risten.

use thiserror::Error;

/// A boxed error type for dynamic error handling.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Errors that can occur during event dispatch.
#[derive(Error, Debug)]
pub enum DispatchError {
    /// An error occurred in a listener or hook.
    #[error(transparent)]
    ListenerError(#[from] BoxError),
}
