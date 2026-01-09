//! Extractor pattern for declarative event data extraction.
//!
//! This module provides the `FromEvent` trait, which allows extracting
//! typed data from events in a composable and type-safe manner.
//!
//! # Overview
//!
//! The Extractor pattern, inspired by Axum and Actix-web, allows handler
//! functions to declare their dependencies as function parameters. The
//! framework automatically extracts the required data from the event.
//!
//! # Example
//!
//! ```rust,ignore
//! use risten::FromEvent;
//!
//! // Define a custom extractor
//! struct Content(String);
//!
//! impl FromEvent<MessageEvent> for Content {
//!     type Error = std::convert::Infallible;
//!
//!     fn from_event(event: &MessageEvent) -> Result<Self, Self::Error> {
//!         Ok(Content(event.content.clone()))
//!     }
//! }
//!
//! // Use in a handler (future macro support)
//! // #[risten::handler]
//! // async fn echo(Content(text): Content) {
//! //     println!("{}", text);
//! // }
//! ```

use std::convert::Infallible;

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

/// A trait for extracting data from an event.
///
/// Implement this trait to create custom extractors that can pull
/// data out of events in a type-safe manner.
///
/// # Type Parameters
///
/// - `E`: The event type to extract from
///
/// # Example
///
/// ```rust
/// use risten::{ExtractError, FromEvent};
///
/// struct UserId(u64);
///
/// #[derive(Clone)]
/// struct UserEvent { user_id: u64 }
///
/// impl FromEvent<UserEvent> for UserId {
///     type Error = std::convert::Infallible;
///
///     fn from_event(event: &UserEvent) -> Result<Self, Self::Error> {
///         Ok(UserId(event.user_id))
///     }
/// }
/// ```
pub trait FromEvent<E>: Sized {
    /// The error type returned if extraction fails.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Attempt to extract `Self` from the given event.
    fn from_event(event: &E) -> Result<Self, Self::Error>;
}

// ============================================================================
// Blanket Implementations
// ============================================================================

/// Option extractor - always succeeds, returns None on failure.
impl<E, T> FromEvent<E> for Option<T>
where
    T: FromEvent<E>,
{
    type Error = Infallible;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        Ok(T::from_event(event).ok())
    }
}

/// Result extractor - passes through the inner extraction.
impl<E, T> FromEvent<E> for Result<T, T::Error>
where
    T: FromEvent<E>,
{
    type Error = Infallible;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        Ok(T::from_event(event))
    }
}

// ============================================================================
// Standard Extractors
// ============================================================================

/// An extractor that clones the entire event.
///
/// Useful when you need the full event data.
#[derive(Debug, Clone)]
pub struct Event<E>(pub E);

impl<E: Clone> FromEvent<E> for Event<E> {
    type Error = Infallible;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        Ok(Event(event.clone()))
    }
}

/// A reference extractor that borrows the entire event.
///
/// Zero-copy extraction for read-only access.
/// Note: This requires the handler to accept a reference lifetime.
impl<'a, E> FromEvent<E> for &'a E
where
    E: 'a,
{
    type Error = Infallible;

    fn from_event(_event: &E) -> Result<Self, Self::Error> {
        // This implementation is a placeholder.
        // Actual borrowed extractors need GAT support in the trait.
        // For now, use Event<E> for owned access.
        unimplemented!("Borrowed extractors require GAT-based FromEvent")
    }
}

// ============================================================================
// Tuple Extractors
// ============================================================================

/// Extract multiple values as a tuple.
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

impl<E, T1, T2, T3> FromEvent<E> for (T1, T2, T3)
where
    T1: FromEvent<E>,
    T2: FromEvent<E>,
    T3: FromEvent<E>,
    T1::Error: 'static,
    T2::Error: 'static,
    T3::Error: 'static,
{
    type Error = ExtractError;

    fn from_event(event: &E) -> Result<Self, Self::Error> {
        let t1 = T1::from_event(event).map_err(|e| ExtractError::new(e.to_string()))?;
        let t2 = T2::from_event(event).map_err(|e| ExtractError::new(e.to_string()))?;
        let t3 = T3::from_event(event).map_err(|e| ExtractError::new(e.to_string()))?;
        Ok((t1, t2, t3))
    }
}

// ============================================================================
// Handler Integration
// ============================================================================

/// A handler that uses extractors to process events.
///
/// This is a bridge between the Extractor pattern and the Handler trait.
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

// Handler impl for single-argument extractor
impl<F, E, T, Out, Fut> crate::Handler<E> for ExtractHandler<F, E, (T,)>
where
    E: crate::core::message::Message,
    T: FromEvent<E> + Send + Sync + 'static,
    T::Error: 'static,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Out> + Send,
    Out: crate::flow::handler::HandlerResult,
{
    type Output = Result<Out, ExtractError>;

    async fn call(&self, input: E) -> Self::Output {
        let arg = T::from_event(&input).map_err(|e| ExtractError::new(e.to_string()))?;
        Ok((self.func)(arg).await)
    }
}

// Handler impl for two-argument extractor
impl<F, E, T1, T2, Out, Fut> crate::flow::handler::Handler<E> for ExtractHandler<F, E, (T1, T2)>
where
    E: crate::core::message::Message,
    T1: FromEvent<E> + Send + Sync + 'static,
    T2: FromEvent<E> + Send + Sync + 'static,
    T1::Error: 'static,
    T2::Error: 'static,
    F: Fn(T1, T2) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Out> + Send,
    Out: crate::flow::handler::HandlerResult,
{
    type Output = Result<Out, ExtractError>;

    async fn call(&self, input: E) -> Self::Output {
        let arg1 = T1::from_event(&input).map_err(|e| ExtractError::new(e.to_string()))?;
        let arg2 = T2::from_event(&input).map_err(|e| ExtractError::new(e.to_string()))?;
        Ok((self.func)(arg1, arg2).await)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestEvent {
        user_id: u64,
        content: String,
    }

    // Custom extractor: UserId
    #[derive(Debug, PartialEq)]
    struct UserId(u64);

    impl FromEvent<TestEvent> for UserId {
        type Error = Infallible;

        fn from_event(event: &TestEvent) -> Result<Self, Self::Error> {
            Ok(UserId(event.user_id))
        }
    }

    // Custom extractor: Content
    #[derive(Debug, PartialEq)]
    struct Content(String);

    impl FromEvent<TestEvent> for Content {
        type Error = Infallible;

        fn from_event(event: &TestEvent) -> Result<Self, Self::Error> {
            Ok(Content(event.content.clone()))
        }
    }

    // Failable extractor
    struct FailExtractor;
    impl FromEvent<TestEvent> for FailExtractor {
        type Error = ExtractError;
        fn from_event(_event: &TestEvent) -> Result<Self, Self::Error> {
            Err(ExtractError::new("simulated failure"))
        }
    }

    #[test]
    fn test_simple_extractor() {
        let event = TestEvent {
            user_id: 42,
            content: "hello".to_string(),
        };

        let user_id = UserId::from_event(&event).unwrap();
        assert_eq!(user_id, UserId(42));

        let content = Content::from_event(&event).unwrap();
        assert_eq!(content, Content("hello".to_string()));
    }

    #[test]
    fn test_option_extractor() {
        let event = TestEvent {
            user_id: 42,
            content: "hello".to_string(),
        };

        let opt: Option<UserId> = FromEvent::from_event(&event).unwrap();
        assert_eq!(opt, Some(UserId(42)));

        let fail: Option<FailExtractor> = FromEvent::from_event(&event).unwrap();
        assert!(fail.is_none());
    }

    #[test]
    fn test_tuple_extractor() {
        let event = TestEvent {
            user_id: 42,
            content: "hello".to_string(),
        };

        let (user_id, content): (UserId, Content) = FromEvent::from_event(&event).unwrap();
        assert_eq!(user_id, UserId(42));
        assert_eq!(content, Content("hello".to_string()));

        // Test tuple failure (3-tuple)
        let res: Result<(UserId, Content, FailExtractor), ExtractError> =
            FromEvent::from_event(&event);
        assert!(res.is_err());
    }

    #[test]
    fn test_event_extractor() {
        let event = TestEvent {
            user_id: 42,
            content: "hello".to_string(),
        };

        let Event(cloned) = Event::from_event(&event).unwrap();
        assert_eq!(cloned, event);
    }

    #[test]
    fn test_extract_error() {
        let err = ExtractError::new("oops");
        assert_eq!(err.message(), "oops");
        assert_eq!(format!("{}", err), "extraction failed: oops");
    }

    #[test]
    fn test_result_extractor() {
        let event = TestEvent {
            user_id: 42,
            content: "hello".to_string(),
        };

        let res: Result<UserId, Infallible> = FromEvent::from_event(&event).unwrap();
        assert!(res.is_ok());

        let res_fail: Result<FailExtractor, ExtractError> = FromEvent::from_event(&event).unwrap();
        assert!(res_fail.is_err());
    }
}
