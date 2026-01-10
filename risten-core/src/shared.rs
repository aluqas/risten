//! # Shared Event Wrapper
//!
//! Provides `SharedEvent<E>` for zero-cost cloning via `Arc`.
//!
//! # Example
//!
//! ```rust,ignore
//! let event = SharedEvent::new(MyEvent { data: "hello".into() });
//! let cloned = event.clone(); // O(1) - only increments reference count
//! ```

use crate::message::Message;
use std::convert::Infallible;
use std::ops::Deref;
use std::sync::Arc;

/// A shared, reference-counted event wrapper.
///
/// `SharedEvent<E>` wraps an event in an `Arc`, making cloning O(1).
/// This is useful when the same event needs to be processed by multiple
/// handlers without copying the underlying data.
///
/// # Performance
///
/// - **Clone**: O(1) - only increments atomic reference count
/// - **Access**: O(1) - direct pointer dereference
///
/// # Example
///
/// ```rust,ignore
/// use risten_core::SharedEvent;
///
/// #[derive(Debug)]
/// struct MyEvent {
///     content: String,
/// }
///
/// let event = SharedEvent::new(MyEvent { content: "Hello".into() });
/// let cloned = event.clone(); // Cheap clone!
/// assert_eq!(event.content, cloned.content);
/// ```
#[derive(Debug)]
pub struct SharedEvent<E>(Arc<E>);

impl<E> SharedEvent<E> {
    /// Create a new shared event.
    pub fn new(event: E) -> Self {
        Self(Arc::new(event))
    }

    /// Get a reference to the inner event.
    pub fn inner(&self) -> &E {
        &self.0
    }

    /// Returns the number of strong references to this event.
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    /// Attempt to unwrap the inner event if this is the only reference.
    ///
    /// Returns `Err(self)` if there are other references.
    pub fn try_unwrap(self) -> Result<E, Self> {
        Arc::try_unwrap(self.0).map_err(SharedEvent)
    }
}

impl<E> Clone for SharedEvent<E> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<E> Deref for SharedEvent<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> AsRef<E> for SharedEvent<E> {
    fn as_ref(&self) -> &E {
        &self.0
    }
}

impl<E: Send + Sync + 'static> Message for SharedEvent<E> {}

// FromEvent for SharedEvent itself (cheap clone)
impl<E: Send + Sync + 'static> crate::context::FromEvent<SharedEvent<E>> for SharedEvent<E> {
    type Error = Infallible;

    fn from_event(event: &SharedEvent<E>) -> Result<Self, Self::Error> {
        Ok(event.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestEvent {
        data: String,
    }

    #[test]
    fn test_shared_event_clone() {
        let event = SharedEvent::new(TestEvent {
            data: "hello".into(),
        });
        assert_eq!(event.strong_count(), 1);

        let cloned = event.clone();
        assert_eq!(event.strong_count(), 2);
        assert_eq!(cloned.strong_count(), 2);
        assert_eq!(event.data, cloned.data);
    }

    #[test]
    fn test_shared_event_deref() {
        let event = SharedEvent::new(TestEvent {
            data: "world".into(),
        });
        assert_eq!(event.data, "world");
    }

    #[test]
    fn test_shared_event_try_unwrap() {
        let event = SharedEvent::new(TestEvent {
            data: "unwrap".into(),
        });
        let inner = event.try_unwrap().expect("should unwrap");
        assert_eq!(inner.data, "unwrap");
    }

    #[test]
    fn test_shared_event_try_unwrap_fails() {
        let event = SharedEvent::new(TestEvent {
            data: "shared".into(),
        });
        let _cloned = event.clone();
        let result = event.try_unwrap();
        assert!(result.is_err());
    }
}
