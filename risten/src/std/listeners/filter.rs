//! Filter Listener - Filters events based on a predicate.

use crate::{core::message::Message, flow::listener::Listener};

/// A `Listener` that filters events based on a predicate.
///
/// Only events for which the predicate returns `true` are passed through.
/// Events that don't match are filtered out (returns `None`).
///
/// # Example
///
/// ```rust,ignore
/// use risten::FilterListener;
///
/// // Only process messages from guilds
/// let guild_only = FilterListener::new(|msg: &DiscordMessage| {
///     msg.guild_id.is_some()
/// });
///
/// // Chain with a handler
/// let pipeline = guild_only.handler(MyHandler);
/// ```
pub struct FilterListener<F> {
    predicate: F,
}

impl<F> FilterListener<F> {
    /// Create a new `FilterListener` with the given predicate.
    ///
    /// The predicate should return `true` for events that should be processed.
    pub fn new(predicate: F) -> Self {
        Self { predicate }
    }
}

impl<E, F> Listener<E> for FilterListener<F>
where
    E: Message + Clone,
    F: Fn(&E) -> bool + Send + Sync + 'static,
{
    type Output = E;

    fn listen(&self, event: &E) -> Option<E> {
        if (self.predicate)(event) {
            Some(event.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestEvent {
        value: i32,
    }

    #[test]
    fn test_filter_passes() {
        let filter = FilterListener::new(|e: &TestEvent| e.value > 5);

        let event = TestEvent { value: 10 };
        let result = filter.listen(&event);
        assert_eq!(result, Some(TestEvent { value: 10 }));
    }

    #[test]
    fn test_filter_rejects() {
        let filter = FilterListener::new(|e: &TestEvent| e.value > 5);

        let event = TestEvent { value: 3 };
        let result = filter.listen(&event);
        assert_eq!(result, None);
    }

    #[test]
    fn test_filter_boundary() {
        let filter = FilterListener::new(|e: &TestEvent| e.value >= 5);

        assert!(filter.listen(&TestEvent { value: 5 }).is_some());
        assert!(filter.listen(&TestEvent { value: 4 }).is_none());
    }
}
