//! Extractor pattern for declarative event data extraction.
//!
//! This module provides traits for extracting data from events:
//! - [`FromEvent`] - Synchronous extraction
//! - [`AsyncFromEvent`] - Asynchronous extraction (for DB lookups, etc.)

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
#[derive(Debug, Clone)]
pub struct Event<E>(pub E);

impl<E: Clone> FromEvent<E> for Event<E> {
    type Error = Infallible;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        Ok(Event(event.clone()))
    }
}

// Tuple Extractors

impl<E, T1, T2> FromEvent<E> for (T1, T2)
where
    T1: FromEvent<E>,
    T2: FromEvent<E>,
    T1::Error: 'static,
    T2::Error: 'static,
{
    type Error = ExtractError;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        let t1 = T1::from_event(event).map_err(|e| ExtractError::new(e.to_string()))?;
        let t2 = T2::from_event(event).map_err(|e| ExtractError::new(e.to_string()))?;
        Ok((t1, t2))
    }
}

// Handler Integration

/// A handler that uses extractors to process events.
pub struct ExtractHandler<F, E, Args> {
    func: F,
    _marker: std::marker::PhantomData<(E, Args)>,
}

impl<F, E, Args> ExtractHandler<F, E, Args> {
    /// Create a new extract handler from a function.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: std::marker::PhantomData,
        }
    }
}

// Handler impl for single-argument extractor (supports async extraction)
impl<F, E, T, Out, Fut> crate::Handler<E> for ExtractHandler<F, E, (T,)>
where
    E: Message + Sync,
    T: AsyncFromEvent<E> + Send + Sync + 'static,
    T::Error: 'static,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Out> + Send,
    Out: crate::handler::HandlerResult,
{
    type Output = Result<Out, ExtractError>;

    async fn call(&self, input: E) -> Self::Output {
        // Use async extraction (works with both FromEvent and AsyncFromEvent)
        let arg = T::from_event(&input)
            .await
            .map_err(|e| ExtractError::new(e.to_string()))?;
        Ok((self.func)(arg).await)
    }
}
