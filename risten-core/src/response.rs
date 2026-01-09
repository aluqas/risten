//! Response conversion traits.

use crate::hook::HookResult;

/// Trait for converting a handler's output into a [`HookResult`].
///
/// # Default Implementations
///
/// - `()` → Stop propagation
/// - `bool` → `true` = Stop, `false` = Next
/// - `HookResult` → As is
/// - `Result<T, E>` → Delegates to inner `T` or propagates error
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `IntoResponse`",
    label = "missing `IntoResponse` implementation",
    note = "IntoResponse must implement the `into_response` method."
)]
pub trait IntoResponse {
    /// Convert the output into propagation behavior and optional error.
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>>;
}

/// Alias for backwards compatibility.
pub use IntoResponse as IntoHookOutcome;

impl IntoResponse for () {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Stop)
    }
}

impl IntoResponse for bool {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(if self {
            HookResult::Stop
        } else {
            HookResult::Next
        })
    }
}

impl IntoResponse for HookResult {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self)
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Ok(t) => t.into_response(),
            Err(e) => Err(Box::new(e)),
        }
    }
}

// Additional common IntoResponse implementations

impl IntoResponse for String {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        // String output means "handled successfully, continue"
        Ok(HookResult::Next)
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Next)
    }
}

impl<T: IntoResponse> IntoResponse for Option<T> {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Some(t) => t.into_response(),
            None => Ok(HookResult::Next),
        }
    }
}
