//! Filter listener for conditional event processing.

use risten_core::{BoxError, Listener, Message};

/// A listener that filters events based on a predicate.
///
/// Returns `Some(event.clone())` if the predicate returns `true`, otherwise `None`.
///
/// # Example
///
/// ```rust,ignore
/// let filter = FilterListener::new(|event: &MyEvent| event.is_important());
/// ```
pub struct FilterListener<F> {
    predicate: F,
}

impl<F> FilterListener<F> {
    /// Create a new filter listener with the given predicate.
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

impl<E, F> Listener<E> for FilterListener<F>
where
    E: Message + Clone + Sync,
    F: Fn(&E) -> bool + Send + Sync + 'static,
{
    type Output = E;

    async fn listen(&self, event: &E) -> Result<Option<Self::Output>, BoxError> {
        if (self.predicate)(event) {
            Ok(Some(event.clone()))
        } else {
            Ok(None)
        }
    }
}

/// A listener that filters events based on an async predicate.
///
/// Use this when your filtering logic requires async operations (e.g., database lookups).
///
/// # Example
///
/// ```rust,ignore
/// let filter = AsyncFilterListener::new(|event: &MyEvent| async move {
///     db.check_allowed(event.user_id).await
/// });
/// ```
pub struct AsyncFilterListener<F> {
    predicate: F,
}

impl<F> AsyncFilterListener<F> {
    /// Create a new async filter listener with the given predicate.
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

impl<E, F, Fut> Listener<E> for AsyncFilterListener<F>
where
    E: Message + Clone + Sync,
    F: Fn(&E) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = bool> + Send,
{
    type Output = E;

    async fn listen(&self, event: &E) -> Result<Option<Self::Output>, BoxError> {
        if (self.predicate)(event).await {
            Ok(Some(event.clone()))
        } else {
            Ok(None)
        }
    }
}
