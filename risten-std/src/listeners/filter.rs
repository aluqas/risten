//! Filter listener for conditional event processing.

use risten_core::{Listener, Message};

/// A listener that filters events based on a predicate.
pub struct FilterListener<F> {
    predicate: F,
}

impl<F> FilterListener<F> {
    /// Create a new filter listener.
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

    fn listen(&self, event: &E) -> Option<Self::Output> {
        if (self.predicate)(event) {
            Some(event.clone())
        } else {
            None
        }
    }
}
