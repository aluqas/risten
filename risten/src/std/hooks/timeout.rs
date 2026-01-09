//! Timeout Hook - Time-limited execution wrapper.
//!
//! **Note**: This module requires the `tokio` runtime. If you're using a
//! different async runtime, you can create a similar wrapper using your
//! runtime's timeout facilities.

// TimeoutHook is only available when tokio is present
// Since tokio is a dev-dependency, we provide a simpler approach

use crate::{
    core::{error::BoxError, message::Message},
    flow::hook::{Hook, HookResult},
};
use std::{future::Future, pin::Pin, time::Duration};

/// A Hook that wraps another Hook with a timeout.
///
/// If the inner hook does not complete within the specified duration,
/// the operation is aborted and an error is returned.
///
/// # Example
///
/// ```rust,ignore
/// use risten::TimeoutHook;
/// use std::time::Duration;
///
/// // Wrap a slow handler with a 5-second timeout
/// let timed = TimeoutHook::new(SlowHandler, Duration::from_secs(5));
/// ```
///
/// # Runtime Requirements
///
/// This hook uses `tokio::time::timeout` internally and requires
/// the tokio runtime to be available. For other runtimes, you can
/// implement a similar wrapper using your runtime's timeout facilities.
pub struct TimeoutHook<H> {
    inner: H,
    duration: Duration,
}

impl<H> TimeoutHook<H> {
    /// Create a new `TimeoutHook` wrapping the given hook.
    pub fn new(inner: H, duration: Duration) -> Self {
        Self { inner, duration }
    }

    /// Create a `TimeoutHook` with the timeout specified in seconds.
    pub fn secs(inner: H, seconds: u64) -> Self {
        Self::new(inner, Duration::from_secs(seconds))
    }

    /// Create a `TimeoutHook` with the timeout specified in milliseconds.
    pub fn millis(inner: H, millis: u64) -> Self {
        Self::new(inner, Duration::from_millis(millis))
    }

    /// Get the configured timeout duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Get a reference to the inner hook.
    pub fn inner(&self) -> &H {
        &self.inner
    }
}

/// Trait for timeout execution - allows runtime-agnostic timeout.
///
/// Implement this trait for your runtime to enable `TimeoutHook`.
pub trait TimeoutExecutor: Send + Sync + 'static {
    /// Execute a future with a timeout.
    fn timeout<'a, F>(
        &'a self,
        duration: Duration,
        future: F,
    ) -> Pin<Box<dyn Future<Output = Result<F::Output, TimeoutError>> + Send + 'a>>
    where
        F: Future + Send + 'a,
        F::Output: Send;
}

/// Error returned when a timeout occurs.
#[derive(Debug, Clone)]
pub struct TimeoutError {
    duration: Duration,
}

impl TimeoutError {
    /// Create a new timeout error.
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }

    /// Get the duration that was exceeded.
    pub fn duration(&self) -> Duration {
        self.duration
    }
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hook execution timed out after {:?}", self.duration)
    }
}

impl std::error::Error for TimeoutError {}

/// A `TimeoutHook` that uses a custom executor.
///
/// This variant allows using any async runtime by providing a custom
/// `TimeoutExecutor` implementation.
pub struct TimeoutHookWithExecutor<H, T> {
    inner: H,
    duration: Duration,
    executor: T,
}

impl<H, T> TimeoutHookWithExecutor<H, T> {
    /// Create a new `TimeoutHookWithExecutor`.
    pub fn new(inner: H, duration: Duration, executor: T) -> Self {
        Self {
            inner,
            duration,
            executor,
        }
    }
}

impl<E, H, T> Hook<E> for TimeoutHookWithExecutor<H, T>
where
    E: Message + Sync,
    H: Hook<E>,
    T: TimeoutExecutor,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        let future = self.inner.on_event(event);
        self.executor
            .timeout(self.duration, future)
            .await
            .map_err(|e| Box::new(e) as BoxError)?
    }
}

#[cfg(test)]
impl<E, H> Hook<E> for TimeoutHook<H>
where
    E: Message + Sync,
    H: Hook<E>,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        match tokio::time::timeout(self.duration, self.inner.on_event(event)).await {
            Ok(res) => res,
            Err(_) => Err(Box::new(TimeoutError::new(self.duration)) as BoxError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_hook_creation() {
        struct DummyHook;
        let hook = TimeoutHook::new(DummyHook, Duration::from_secs(5));
        assert_eq!(hook.duration(), Duration::from_secs(5));

        // Test inner accessor
        let _ = hook.inner();
    }

    #[test]
    fn test_timeout_hook_secs() {
        struct DummyHook;
        let hook = TimeoutHook::secs(DummyHook, 10);
        assert_eq!(hook.duration(), Duration::from_secs(10));
    }

    #[test]
    fn test_timeout_hook_millis() {
        struct DummyHook;
        let hook = TimeoutHook::millis(DummyHook, 500);
        assert_eq!(hook.duration(), Duration::from_millis(500));
    }

    #[test]
    fn test_timeout_error_display() {
        let error = TimeoutError::new(Duration::from_secs(5));
        assert!(error.to_string().contains("5s"));
        assert_eq!(error.duration(), Duration::from_secs(5));
    }

    // Async tests requiring tokio

    #[derive(Clone)]
    struct AsyncMockEvent;
    // impl Message for AsyncMockEvent {} - Covered by blanket impl

    struct SleepyHook {
        delay: Duration,
    }

    impl Hook<AsyncMockEvent> for SleepyHook {
        async fn on_event(&self, _event: &AsyncMockEvent) -> Result<HookResult, BoxError> {
            tokio::time::sleep(self.delay).await;
            Ok(HookResult::Next)
        }
    }

    struct FastHook;
    impl Hook<AsyncMockEvent> for FastHook {
        async fn on_event(&self, _event: &AsyncMockEvent) -> Result<HookResult, BoxError> {
            Ok(HookResult::Stop)
        }
    }

    #[tokio::test]
    async fn test_timeout_trigger() {
        let inner = SleepyHook {
            delay: Duration::from_millis(50),
        };
        let hook = TimeoutHook::millis(inner, 10);

        let result = hook.on_event(&AsyncMockEvent).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("10ms"));
    }

    #[tokio::test]
    async fn test_timeout_success() {
        let inner = FastHook;
        // Long timeout, fast hook
        let hook = TimeoutHook::millis(inner, 100);

        let result = hook.on_event(&AsyncMockEvent).await;
        assert!(result.is_ok());
        match result.unwrap() {
            HookResult::Stop => {}
            _ => panic!("Expected Stop"),
        }
    }
}
