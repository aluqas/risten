//! Timeout hook for time-limited execution.

use risten_core::{BoxError, Hook, HookResult, Message};
use std::time::Duration;
use tokio::time::timeout;

/// Error returned when a hook times out.
#[derive(Debug, Clone)]
pub struct TimeoutError;

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hook execution timed out")
    }
}

impl std::error::Error for TimeoutError {}

/// A hook that wraps another hook with a timeout.
pub struct TimeoutHook<H> {
    inner: H,
    duration: Duration,
}

impl<H> TimeoutHook<H> {
    /// Create a new timeout hook.
    pub fn new(inner: H, duration: Duration) -> Self {
        Self { inner, duration }
    }
}

impl<E: Message + Sync, H: Hook<E>> Hook<E> for TimeoutHook<H> {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        match timeout(self.duration, self.inner.on_event(event)).await {
            Ok(result) => result,
            Err(_) => Err(Box::new(TimeoutError)),
        }
    }
}
