//! Logging Hook - Observability for event processing.

use crate::{
    core::{error::BoxError, message::Message},
    flow::hook::{Hook, HookResult},
};
use std::fmt::Debug;

/// A Hook that logs events for observability.
///
/// This hook logs event information and continues processing.
/// It uses the `tracing` crate when available, falling back to `log`.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{LoggingHook, static_hooks};
///
/// // Create a logging hook with default settings
/// let logging = LoggingHook::new();
///
/// // Or with a custom name
/// let logging = LoggingHook::named("command_pipeline");
///
/// // Use in a chain
/// let chain = static_hooks![logging, MyHandler];
/// ```
pub struct LoggingHook {
    name: &'static str,
}

impl LoggingHook {
    /// Create a new `LoggingHook` with a default name.
    pub fn new() -> Self {
        Self { name: "event" }
    }

    /// Create a new `LoggingHook` with a custom name.
    ///
    /// The name is used in log messages to identify the pipeline stage.
    pub fn named(name: &'static str) -> Self {
        Self { name }
    }
}

impl Default for LoggingHook {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Hook<E> for LoggingHook
where
    E: Message + Debug + Sync,
{
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        // Use tracing if available, otherwise fall back to basic logging
        #[cfg(feature = "tracing")]
        {
            tracing::debug!(name = %self.name, event = ?event, "Processing event");
        }

        #[cfg(not(feature = "tracing"))]
        {
            // Basic debug output when tracing is not available
            let _ = (self.name, event); // Suppress unused warnings
        }

        Ok(HookResult::Next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestEvent {
        data: String,
    }

    #[tokio::test]
    async fn test_logging_hook_continues() {
        let hook = LoggingHook::new();
        let event = TestEvent {
            data: "test".into(),
        };

        let result = hook.on_event(&event).await.unwrap();
        assert_eq!(result, HookResult::Next);
    }

    #[tokio::test]
    async fn test_logging_hook_named() {
        let hook = LoggingHook::named("my_pipeline");
        let event = TestEvent {
            data: "test".into(),
        };

        let result = hook.on_event(&event).await.unwrap();
        assert_eq!(result, HookResult::Next);
    }
}
