use crate::flow::hook::HookResult;

/// Trait for converting a handler's output into a HookResult.
///
/// Implement this for your handler output types to control
/// how the pipeline interprets the result.
///
/// # Default Implementations
///
/// - `()` → Stop propagation (backwards compatible)
/// - `bool` → `true` = Stop, `false` = Continue
/// - `HookResult` → As is
/// - `Result<T, E>` → Delegates to inner `T` or propagates error
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `IntoResponse`",
    label = "missing `IntoResponse` implementation",
    note = "IntoResponse must implement the `into_response` method to convert the output into propagation behavior and optional error."
)]
pub trait IntoResponse {
    /// Convert the output into propagation behavior and optional error.
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>>;
}

// Alias for backwards compatibility if needed, or just use IntoResponse
pub use IntoResponse as IntoHookOutcome;

impl IntoResponse for () {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Stop) // Stop propagation by default for void handlers
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

// Implement for Option (Some = Next/Stop based on inner, None = Next?)
// Actually usually Option<T> might mean "if Some, handle it".
// Let's stick to simple ones first.
