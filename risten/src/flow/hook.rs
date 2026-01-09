use crate::core::message::Message;
use std::{future::Future, pin::Pin};

/// Result of hook execution indicating whether to continue or stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    /// The event was handled or observed, continue to the next hook.
    Next,
    /// Stop propagation of the event to subsequent hooks.
    Stop,
}

/// A low-level primitive for injecting logic into the event processing pipeline (Layer 0).
///
/// Hooks are the fundamental building blocks of the `sakuramiya-event` system.
/// They can be used for:
/// - Observing events (logging, metrics)
/// - Filtering events
/// - Executing pipelines (wrapping Listener + Handler)
///
/// # Example
///
/// ```rust
/// use risten::{Hook, HookResult, Message};
///
/// #[derive(Clone)]
/// struct MyEvent { data: String }
///
/// struct LoggingHook;
///
/// impl Hook<MyEvent> for LoggingHook {
///     async fn on_event(
///         &self,
///         event: &MyEvent,
///     ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
///         println!("Event received: {}", event.data);
///         Ok(HookResult::Next)  // Continue to next hook
///     }
/// }
/// ```
///
/// This trait uses native `async fn` for zero-cost static dispatch.
/// For dynamic dispatch (e.g. in Registry), use `DynHook`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not implement `Hook<{E}>`",
    label = "missing `Hook` implementation",
    note = "Hooks must implement `on_event` for the specific event type `{E}`."
)]
pub trait Hook<E: Message>: Send + Sync + 'static {
    /// Called when an event is dispatched.
    fn on_event(
        &self,
        event: &E,
    ) -> impl Future<Output = Result<HookResult, Box<dyn std::error::Error + Send + Sync>>> + Send;
}

/// Dynamic object-safe version of `Hook`.
///
/// Use this trait when you need runtime polymorphism (e.g., in a Registry).
pub trait DynHook<E: Message>: Send + Sync + 'static {
    fn on_event_dyn<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<HookResult, Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + 'a,
        >,
    >;
}
// Blanket impl for boxed DynHook to allow it to be used where Hook is expected if needed,
// Blanket implementation: Any type implementing Hook implements DynHook automatically.
impl<E: Message, T: Hook<E>> DynHook<E> for T {
    fn on_event_dyn<'a>(
        &'a self,
        event: &'a E,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<HookResult, Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(self.on_event(event))
    }
}

// Allow Box<dyn DynHook> to be used where Hook is expected.
impl<E: Message> Hook<E> for Box<dyn DynHook<E>> {
    async fn on_event(
        &self,
        event: &E,
    ) -> Result<HookResult, Box<dyn std::error::Error + Send + Sync>> {
        self.on_event_dyn(event).await
    }
}

// ============================================================================
// Enum Dispatch Macro (RFC 0004 - Static Optimization)
// ============================================================================

/// Generate an enum that dispatches to inner Hook implementations via match.
///
/// This macro creates an enum where each variant wraps a type implementing `Hook<E>`,
/// and implements `Hook<E>` for the enum itself by dispatching via a match statement.
/// This eliminates vtable overhead entirely - the compiler can inline all branches.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{enum_hook, Hook, HookResult, Message};
///
/// struct LoggingHook;
/// struct MetricsHook;
/// struct AuthHook;
///
/// impl Hook<MyEvent> for LoggingHook { ... }
/// impl Hook<MyEvent> for MetricsHook { ... }
/// impl Hook<MyEvent> for AuthHook { ... }
///
/// // Generate the enum and Hook impl
/// enum_hook! {
///     /// My combined hook enum
///     pub enum MyHooks<MyEvent> {
///         Logging(LoggingHook),
///         Metrics(MetricsHook),
///         Auth(AuthHook),
///     }
/// }
///
/// // Use like any Hook
/// let hook = MyHooks::Logging(LoggingHook);
/// hook.on_event(&my_event).await?;
/// ```
///
/// # Generated Code
///
/// The macro expands to:
/// - The enum definition with the specified variants
/// - `impl Hook<E> for EnumName` with a match dispatching to each variant
/// - `impl From<VariantType> for EnumName` for each variant
#[macro_export]
macro_rules! enum_hook {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident<$event:ty> {
            $(
                $variant:ident($inner:ty)
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $(
                $variant($inner),
            )+
        }

        impl $crate::Hook<$event> for $name {
            async fn on_event(
                &self,
                event: &$event,
            ) -> Result<$crate::HookResult, Box<dyn std::error::Error + Send + Sync>> {
                match self {
                    $(
                        Self::$variant(inner) => inner.on_event(event).await,
                    )+
                }
            }
        }

        // Generate From impls for ergonomic construction
        $(
            impl From<$inner> for $name {
                fn from(inner: $inner) -> Self {
                    Self::$variant(inner)
                }
            }
        )+
    };
}
