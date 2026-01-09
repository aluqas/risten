//! Logging hook for event observation.

use risten_core::{BoxError, Hook, HookResult, Message};

/// A hook that logs events for debugging/observation.
pub struct LoggingHook;

impl<E: Message + std::fmt::Debug> Hook<E> for LoggingHook {
    async fn on_event(&self, event: &E) -> Result<HookResult, BoxError> {
        #[cfg(feature = "tracing")]
        {
            tracing::info!(?event, "Processing event");
        }
        #[cfg(not(feature = "tracing"))]
        {
            let _ = event; // Suppress unused warning
        }
        Ok(HookResult::Next)
    }
}
