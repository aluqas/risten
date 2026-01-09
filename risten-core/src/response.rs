//! Response conversion traits.
//!
//! This module provides the [`IntoResponse`] trait for converting handler outputs
//! into [`HookResult`] values that control event propagation.
//!
//! # Semantic Markers
//!
//! Use [`Handled`] and [`Continue`] to explicitly express intent:
//!
//! ```rust,ignore
//! fn my_handler(event: MyEvent) -> Handled {
//!     // Process event...
//!     Handled  // Explicitly stops propagation
//! }
//!
//! fn observer_handler(event: MyEvent) -> Continue<String> {
//!     // Log and pass through
//!     Continue("logged".to_string())
//! }
//! ```

use crate::hook::HookResult;

/// Trait for converting a handler's output into a [`HookResult`].
///
/// This trait enables flexible handler return types. The framework provides
/// implementations for common types:
///
/// # Default Implementations
///
/// | Type | Behavior |
/// |------|----------|
/// | `()` | Stop propagation (event handled) |
/// | `bool` | `true` = Stop, `false` = Next |
/// | [`HookResult`] | As is |
/// | [`Handled`] | Stop propagation |
/// | [`Continue<T>`] | Next (continue propagation) |
/// | `Result<T, E>` | Delegates to `T` or propagates error |
/// | `Option<T>` | `Some(t)` delegates, `None` = Next |
/// | `String` / `&str` | Next (informational output) |
/// | Numeric types | Next (status codes, counts) |
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

/// A marker type indicating the event was fully handled.
///
/// Returns `HookResult::Stop` when converted to a response.
///
/// # Example
///
/// ```rust,ignore
/// fn command_handler(cmd: Command) -> Handled {
///     execute_command(cmd);
///     Handled
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Handled;

impl IntoResponse for Handled {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Stop)
    }
}

/// A wrapper type indicating the event should continue propagating.
///
/// Wraps an inner value (for logging, metrics, etc.) while returning
/// `HookResult::Next`.
///
/// # Example
///
/// ```rust,ignore
/// fn logging_handler(event: MyEvent) -> Continue<()> {
///     log::info!("Event received: {:?}", event);
///     Continue(())
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Continue<T = ()>(pub T);

impl<T> IntoResponse for Continue<T> {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Next)
    }
}

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

impl<T: IntoResponse> IntoResponse for Option<T> {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            Some(t) => t.into_response(),
            None => Ok(HookResult::Next),
        }
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Next)
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        Ok(HookResult::Next)
    }
}

macro_rules! impl_into_response_for_numeric {
    ($($ty:ty),*) => {
        $(
            impl IntoResponse for $ty {
                fn into_response(self) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
                    Ok(HookResult::Next)
                }
            }
        )*
    };
}

impl_into_response_for_numeric!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);
