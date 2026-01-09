//! # Context Extraction (Context Layer Support)
//!
//! Provides the extractor pattern for declarative event data extraction,
//! enabling the "Context injection" feature of the Handler layer.
//!
//! This module is part of **Layer 4 (Context)** in the Risten architecture.
//! It allows user-defined handler functions to receive extracted context
//! (e.g., user data, permissions, parsed commands) without manual boilerplate.
//!
//! # Extractors
//!
//! - [`FromEvent`] - Synchronous extraction (simple field access, parsing)
//! - [`AsyncFromEvent`] - Asynchronous extraction (DB lookups, API calls)
//!
//! # Handler Integration
//!
//! Use [`ExtractHandler`] to wrap functions that accept extractors as arguments:
//!
//! ```rust,ignore
//! // Function signature defines what to extract
//! async fn my_handler(user: UserContext, cmd: ParsedCommand) -> Result<(), Error> {
//!     // user and cmd are automatically extracted from the event
//! }
//!
//! // Wrap with ExtractHandler to use with Listener
//! let handler = ExtractHandler::new(my_handler);
//! ```

use crate::message::Message;
use std::convert::Infallible;
use std::future::Future;

/// Error type for extraction failures.
#[derive(Debug)]
pub struct ExtractError {
    message: String,
}

impl ExtractError {
    /// Create a new extraction error with a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "extraction failed: {}", self.message)
    }
}

impl std::error::Error for ExtractError {}

/// A trait for extracting data from an event synchronously.
///
/// Use this for simple, non-blocking extractions.
/// For async extractions (e.g., database lookups), use [`AsyncFromEvent`].
pub trait FromEvent<E>: Sized {
    /// The error type returned if extraction fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Attempt to extract `Self` from the given event.
    fn from_event(event: &E) -> Result<Self, Self::Error>;
}

/// A trait for extracting data from an event asynchronously.
///
/// Use this when extraction requires async operations such as:
/// - Database lookups
/// - External API calls
/// - Cache queries
///
/// # Example
///
/// ```rust,ignore
/// struct UserContext {
///     user: User,
///     permissions: Vec<Permission>,
/// }
///
/// impl AsyncFromEvent<MessageEvent> for UserContext {
///     type Error = DbError;
///
///     async fn from_event(event: &MessageEvent) -> Result<Self, Self::Error> {
///         let user = db.get_user(event.author_id).await?;
///         let permissions = db.get_permissions(user.id).await?;
///         Ok(UserContext { user, permissions })
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be asynchronously extracted from `{E}`",
    label = "missing `AsyncFromEvent` implementation",
    note = "Implement `AsyncFromEvent<{E}>` to enable async extraction."
)]
pub trait AsyncFromEvent<E>: Sized + Send {
    /// The error type returned if extraction fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Asynchronously extract `Self` from the given event.
    fn from_event(event: &E) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

// Blanket implementation: Any FromEvent automatically implements AsyncFromEvent
impl<E, T> AsyncFromEvent<E> for T
where
    T: FromEvent<E> + Send,
    E: Send + Sync,
{
    type Error = T::Error;

    async fn from_event(event: &E) -> Result<Self, Self::Error> {
        T::from_event(event)
    }
}

// Blanket Implementations

impl<E, T> FromEvent<E> for Option<T>
where
    T: FromEvent<E>,
{
    type Error = Infallible;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        Ok(T::from_event(event).ok())
    }
}

impl<E, T> FromEvent<E> for Result<T, T::Error>
where
    T: FromEvent<E>,
{
    type Error = Infallible;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        Ok(T::from_event(event))
    }
}

// Standard Extractors

/// An extractor that clones the entire event.
///
/// Use this when you need the full event as an owned value in your handler.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Event<E>(pub E);

impl<E: Clone> FromEvent<E> for Event<E> {
    type Error = Infallible;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        Ok(Event(event.clone()))
    }
}

// Tuple Extractors

/// Macro to implement FromEvent for tuples of extractors.
macro_rules! impl_from_event_tuple {
    ($($T:ident),+) => {
        impl<E, $($T,)+> FromEvent<E> for ($($T,)+)
        where
            $(
                $T: FromEvent<E>,
                $T::Error: 'static,
            )+
        {
            type Error = ExtractError;

            #[allow(non_snake_case)]
            fn from_event(event: &E) -> Result<Self, Self::Error> {
                $(
                    let $T = $T::from_event(event)
                        .map_err(|e| ExtractError::new(e.to_string()))?;
                )+
                Ok(($($T,)+))
            }
        }
    };
}

impl_from_event_tuple!(T1);
impl_from_event_tuple!(T1, T2);
impl_from_event_tuple!(T1, T2, T3);
impl_from_event_tuple!(T1, T2, T3, T4);
impl_from_event_tuple!(T1, T2, T3, T4, T5);
impl_from_event_tuple!(T1, T2, T3, T4, T5, T6);
impl_from_event_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_from_event_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_from_event_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_from_event_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_from_event_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_from_event_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

// Handler Integration

/// A handler that uses extractors to process events (async version).
///
/// `ExtractHandler` wraps a user function and automatically extracts
/// arguments from the event using the [`AsyncFromEvent`] trait.
///
/// # Multi-Argument Support
///
/// Supports functions with 0 to 12 extractor arguments:
///
/// ```rust,ignore
/// // 0 arguments - just receives the event
/// ExtractHandler::new(|| async { Ok(()) });
///
/// // 1 argument
/// ExtractHandler::new(|user: User| async move { ... });
///
/// // 2 arguments
/// ExtractHandler::new(|user: User, cmd: Command| async move { ... });
///
/// // Up to 12 arguments supported
/// ```
///
/// For synchronous functions, use [`SyncExtractHandler`].
pub struct ExtractHandler<F, E, Args> {
    func: F,
    _marker: std::marker::PhantomData<(E, Args)>,
}

impl<F, E, Args> ExtractHandler<F, E, Args> {
    /// Create a new extract handler from an async function.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: std::marker::PhantomData,
        }
    }
}

/// A handler that uses extractors to process events (sync version).
///
/// `SyncExtractHandler` wraps a synchronous user function and automatically
/// extracts arguments from the event using the [`FromEvent`] trait.
///
/// # Example
///
/// ```rust,ignore
/// // Synchronous handler with extractors
/// fn my_sync_handler(user: User, cmd: Command) -> Result<(), Error> {
///     // Synchronous business logic
///     Ok(())
/// }
///
/// let handler = SyncExtractHandler::new(my_sync_handler);
/// ```
///
/// For asynchronous functions, use [`ExtractHandler`].
pub struct SyncExtractHandler<F, E, Args> {
    func: F,
    _marker: std::marker::PhantomData<(E, Args)>,
}

impl<F, E, Args> SyncExtractHandler<F, E, Args> {
    /// Create a new sync extract handler from a synchronous function.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Macro to implement Handler for ExtractHandler with N arguments.
macro_rules! impl_extract_handler {
    // Base case: 0 arguments
    () => {
        impl<F, E, Out, Fut> crate::Handler<E> for ExtractHandler<F, E, ()>
        where
            E: Message + Sync,
            F: Fn() -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Out> + Send,
            Out: crate::handler::HandlerResult,
        {
            type Output = Out;

            async fn call(&self, _input: E) -> Self::Output {
                (self.func)().await
            }
        }
    };

    // Recursive case: 1+ arguments
    ($($T:ident),+) => {
        impl<F, E, $($T,)+ Out, Fut> crate::Handler<E> for ExtractHandler<F, E, ($($T,)+)>
        where
            E: Message + Sync,
            $(
                $T: AsyncFromEvent<E> + Send + Sync + 'static,
                $T::Error: 'static,
            )+
            F: Fn($($T,)+) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = Out> + Send,
            Out: crate::handler::HandlerResult,
        {
            type Output = Result<Out, ExtractError>;

            #[allow(non_snake_case)]
            async fn call(&self, input: E) -> Self::Output {
                $(
                    let $T = $T::from_event(&input)
                        .await
                        .map_err(|e| ExtractError::new(e.to_string()))?;
                )+
                Ok((self.func)($($T,)+).await)
            }
        }
    };
}

impl_extract_handler!();
impl_extract_handler!(T1);
impl_extract_handler!(T1, T2);
impl_extract_handler!(T1, T2, T3);
impl_extract_handler!(T1, T2, T3, T4);
impl_extract_handler!(T1, T2, T3, T4, T5);
impl_extract_handler!(T1, T2, T3, T4, T5, T6);
impl_extract_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

/// Macro to implement Handler for SyncExtractHandler with N arguments.
macro_rules! impl_sync_extract_handler {
    // Base case: 0 arguments
    () => {
        impl<F, E, Out> crate::Handler<E> for SyncExtractHandler<F, E, ()>
        where
            E: Message + Sync,
            F: Fn() -> Out + Send + Sync + 'static,
            Out: crate::handler::HandlerResult,
        {
            type Output = Out;

            async fn call(&self, _input: E) -> Self::Output {
                (self.func)()
            }
        }
    };

    // Recursive case: 1+ arguments (sync extraction only)
    ($($T:ident),+) => {
        impl<F, E, $($T,)+ Out> crate::Handler<E> for SyncExtractHandler<F, E, ($($T,)+)>
        where
            E: Message + Sync,
            $(
                $T: FromEvent<E> + Send + Sync + 'static,
                $T::Error: 'static,
            )+
            F: Fn($($T,)+) -> Out + Send + Sync + 'static,
            Out: crate::handler::HandlerResult,
        {
            type Output = Result<Out, ExtractError>;

            #[allow(non_snake_case)]
            async fn call(&self, input: E) -> Self::Output {
                $(
                    let $T = $T::from_event(&input)
                        .map_err(|e| ExtractError::new(e.to_string()))?;
                )+
                Ok((self.func)($($T,)+))
            }
        }
    };
}

impl_sync_extract_handler!();
impl_sync_extract_handler!(T1);
impl_sync_extract_handler!(T1, T2);
impl_sync_extract_handler!(T1, T2, T3);
impl_sync_extract_handler!(T1, T2, T3, T4);
impl_sync_extract_handler!(T1, T2, T3, T4, T5);
impl_sync_extract_handler!(T1, T2, T3, T4, T5, T6);
impl_sync_extract_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_sync_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_sync_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_sync_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_sync_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_sync_extract_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
