//! Map Listener - Transforms events using a mapper function.

use crate::{core::message::Message, flow::listener::Listener};

/// A `Listener` that transforms events using a mapper function.
///
/// Every input event is transformed to a new output event.
/// This is a pure transformation - it never filters events.
///
/// # Example
///
/// ```rust,ignore
/// use risten::MapListener;
///
/// // Extract just the content field from messages
/// let extract_content = MapListener::new(|msg: &DiscordMessage| {
///     msg.content.clone()
/// });
///
/// // Chain with a handler that works with String
/// let pipeline = extract_content.handler(StringHandler);
/// ```
pub struct MapListener<F> {
    mapper: F,
}

impl<F> MapListener<F> {
    /// Create a new `MapListener` with the given mapper function.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct InputEvent {
        value: i32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct OutputEvent {
        doubled: i32,
    }

    #[test]
    fn test_map_transforms() {
        let mapper = MapListener::new(|e: &InputEvent| OutputEvent {
            doubled: e.value * 2,
        });

        let input = InputEvent { value: 5 };
        let result = mapper.listen(&input);
        assert_eq!(result, Some(OutputEvent { doubled: 10 }));
    }

    #[test]
    fn test_map_always_returns_some() {
        let mapper = MapListener::new(|e: &InputEvent| OutputEvent {
            doubled: e.value * 2,
        });

        // Map should always return Some
        assert!(mapper.listen(&InputEvent { value: 0 }).is_some());
        assert!(mapper.listen(&InputEvent { value: -1 }).is_some());
        assert!(mapper.listen(&InputEvent { value: 100 }).is_some());
    }

    #[test]
    fn test_map_to_same_type() {
        let mapper = MapListener::new(|e: &InputEvent| InputEvent { value: e.value + 1 });

        let result = mapper.listen(&InputEvent { value: 10 });
        assert_eq!(result.unwrap().value, 11);
    }
}
