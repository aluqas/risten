use crate::core::message::Message;
use std::future::Future;

/// A marker trait for the result of an endpoint execution.
///
/// This is typically a `Result` or `Action`.
pub trait HandlerResult: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> HandlerResult for T {}

// IntoHookOutcome is moved to crate::response
pub use crate::core::response::IntoResponse as IntoHookOutcome;

/// A handler represents the final destination of an event processing pipeline (Phase 2).
///
/// It receives a fully owned message (Trigger) and performs async work.
///
/// # Example
///
/// ```rust
/// use risten::{Handler, Message};
///
/// #[derive(Clone)]
/// struct MyTrigger { command: String }
///
/// struct CommandHandler;
///
/// impl Handler<MyTrigger> for CommandHandler {
///     type Output = ();
///
///     async fn call(&self, input: MyTrigger) -> Self::Output {
///         println!("Handling command: {}", input.command);
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot handle input of type `{In}`",
    label = "missing `Handler<{In}>` implementation",
    note = "Handlers must implement the `call` method for the input type `{In}`."
)]
pub trait Handler<In: Message>: Send + Sync + 'static {
    /// The output type of the handler, usually `()`, `Result`, or `Action`.
    type Output: HandlerResult;

    /// Executes the handler logic.
    fn call(&self, input: In) -> impl Future<Output = Self::Output> + Send;
}

impl<F, In, Out, Fut> Handler<In> for F
where
    In: Message,
    Out: HandlerResult,
    F: Fn(In) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Out> + Send,
{
    type Output = Out;

    fn call(&self, input: In) -> impl Future<Output = Self::Output> + Send {
        (self)(input)
    }
}

// ============================================================================
// Enum Dispatch Macro for Handlers
// ============================================================================

/// Generate an enum that dispatches to inner Handler implementations via match.
///
/// This macro creates an enum where each variant wraps a type implementing `Handler<In>`,
/// and implements `Handler<In>` for the enum itself by dispatching via a match statement.
/// This eliminates vtable overhead entirely - the compiler can inline all branches.
///
/// All variants must have the same `Output` type.
///
/// # Example
///
/// ```rust,ignore
/// use risten::{enum_handler, Handler, Message};
///
/// struct EchoHandler;
/// struct PingHandler;
///
/// impl Handler<MyTrigger> for EchoHandler {
///     type Output = ();
///     async fn call(&self, input: MyTrigger) { println!("Echo: {:?}", input); }
/// }
///
/// impl Handler<MyTrigger> for PingHandler {
///     type Output = ();
///     async fn call(&self, input: MyTrigger) { println!("Pong!"); }
/// }
///
/// // Generate the enum and Handler impl
/// enum_handler! {
///     /// My combined handler enum
///     pub enum MyHandlers<MyTrigger, Output = ()> {
///         Echo(EchoHandler),
///         Ping(PingHandler),
///     }
/// }
///
/// // Use like any Handler
/// let handler = MyHandlers::Echo(EchoHandler);
/// handler.call(my_trigger).await;
/// ```
///
/// # Generated Code
///
/// The macro expands to:
/// - The enum definition with the specified variants
/// - `impl Handler<In> for EnumName` with a match dispatching to each variant
/// - `impl From<VariantType> for EnumName` for each variant
#[macro_export]
macro_rules! enum_handler {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident<$input:ty, Output = $output:ty> {
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

        impl $crate::Handler<$input> for $name {
            type Output = $output;

            async fn call(&self, input: $input) -> Self::Output {
                match self {
                    $(
                        Self::$variant(inner) => inner.call(input).await,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestTrigger {
        value: i32,
    }

    struct DoubleHandler;
    struct TripleHandler;

    impl Handler<TestTrigger> for DoubleHandler {
        type Output = i32;

        async fn call(&self, input: TestTrigger) -> i32 {
            input.value * 2
        }
    }

    impl Handler<TestTrigger> for TripleHandler {
        type Output = i32;

        async fn call(&self, input: TestTrigger) -> i32 {
            input.value * 3
        }
    }

    enum_handler! {
        /// Test handler enum
        pub enum MathHandlers<TestTrigger, Output = i32> {
            Double(DoubleHandler),
            Triple(TripleHandler),
        }
    }

    #[tokio::test]
    async fn test_enum_handler_double() {
        let handler = MathHandlers::Double(DoubleHandler);
        let result = handler.call(TestTrigger { value: 5 }).await;
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_enum_handler_triple() {
        let handler = MathHandlers::Triple(TripleHandler);
        let result = handler.call(TestTrigger { value: 5 }).await;
        assert_eq!(result, 15);
    }

    #[tokio::test]
    async fn test_enum_handler_from() {
        let handler: MathHandlers = DoubleHandler.into();
        let result = handler.call(TestTrigger { value: 7 }).await;
        assert_eq!(result, 14);
    }
}
