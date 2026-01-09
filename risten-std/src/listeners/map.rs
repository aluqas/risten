//! Map listener for event transformation.

use risten_core::{BoxError, Listener, Message};

/// A listener that transforms events using a synchronous mapper function.
///
/// The mapper always succeeds (returns `Some`), making this suitable for
/// simple transformations that don't need to filter.
///
/// # Example
///
/// ```rust,ignore
/// let mapper = MapListener::new(|event: &RawEvent| ProcessedEvent {
///     id: event.id,
///     content: event.content.to_uppercase(),
/// });
/// ```
pub struct MapListener<F> {
    mapper: F,
}

impl<F> MapListener<F> {
    /// Create a new map listener with the given mapper function.
    pub fn new(mapper: F) -> Self {
        Self { mapper }
    }
}

impl<In, Out, F> Listener<In> for MapListener<F>
where
    In: Message + Sync,
    Out: Message,
    F: Fn(&In) -> Out + Send + Sync + 'static,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Out>, BoxError> {
        Ok(Some((self.mapper)(event)))
    }
}

/// A listener that transforms events using an async mapper function.
///
/// Use this when your transformation logic requires async operations.
///
/// # Example
///
/// ```rust,ignore
/// let mapper = AsyncMapListener::new(|event: &RawEvent| async move {
///     let enriched_data = db.get_user_info(event.user_id).await;
///     EnrichedEvent { event: event.clone(), user: enriched_data }
/// });
/// ```
pub struct AsyncMapListener<F> {
    mapper: F,
}

impl<F> AsyncMapListener<F> {
    /// Create a new async map listener with the given mapper function.
    pub fn new(mapper: F) -> Self {
        Self { mapper }
    }
}

impl<In, Out, F, Fut> Listener<In> for AsyncMapListener<F>
where
    In: Message + Sync,
    Out: Message,
    F: Fn(&In) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Out> + Send,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Out>, BoxError> {
        Ok(Some((self.mapper)(event).await))
    }
}

/// A listener that optionally transforms events.
///
/// Unlike `MapListener`, this allows the mapper to return `None` to filter events.
///
/// # Example
///
/// ```rust,ignore
/// let mapper = TryMapListener::new(|event: &RawEvent| {
///     if event.is_valid() {
///         Some(ProcessedEvent::from(event))
///     } else {
///         None
///     }
/// });
/// ```
pub struct TryMapListener<F> {
    mapper: F,
}

impl<F> TryMapListener<F> {
    /// Create a new try-map listener with the given mapper function.
    pub fn new(mapper: F) -> Self {
        Self { mapper }
    }
}

impl<In, Out, F> Listener<In> for TryMapListener<F>
where
    In: Message + Sync,
    Out: Message,
    F: Fn(&In) -> Option<Out> + Send + Sync + 'static,
{
    type Output = Out;

    async fn listen(&self, event: &In) -> Result<Option<Out>, BoxError> {
        Ok((self.mapper)(event))
    }
}
