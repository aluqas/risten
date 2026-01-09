//! Map listener for event transformation.

use risten_core::{Listener, Message};

/// A listener that transforms events.
pub struct MapListener<F> {
    mapper: F,
}

impl<F> MapListener<F> {
    /// Create a new map listener.
    pub fn new(mapper: F) -> Self {
        Self { mapper }
    }
}

impl<In, Out, F> Listener<In> for MapListener<F>
where
    In: Message,
    Out: Message,
    F: Fn(&In) -> Out + Send + Sync + 'static,
{
    type Output = Out;

    fn listen(&self, event: &In) -> Option<Out> {
        Some((self.mapper)(event))
    }
}
