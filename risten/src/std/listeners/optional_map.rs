//! Optional Map Listener - Conditionally transforms events (filter_map).

use crate::{core::message::Message, flow::listener::Listener};

/// A `Listener` that conditionally transforms events.
///
/// This is equivalent to `filter_map` - events can be both filtered and
/// transformed in a single step. The mapper returns `Some(output)` to
/// pass the transformed event, or `None` to filter it out.
///
/// # Example
///
/// ```rust,ignore
/// use risten::OptionalMapListener;
///
/// // Parse command from message, filtering non-commands
/// let command_parser = OptionalMapListener::new(|msg: &Message| {
///     if msg.content.starts_with("!") {
///         Some(Command::parse(&msg.content[1..]))
///     } else {
///         None
///     }
/// });
/// ```
pub struct OptionalMapListener<F> {
    mapper: F,
}

impl<F> OptionalMapListener<F> {
    /// Create a new `OptionalMapListener` with the given mapper function.
    ///
    /// The mapper should return `Some(output)` to pass the event through,
    /// or `None` to filter it out.
    pub fn new(mapper: F) -> Self {
        Self { mapper }
    }
}

impl<In, Out, F> Listener<In> for OptionalMapListener<F>
where
    In: Message,
    Out: Message,
    F: Fn(&In) -> Option<Out> + Send + Sync + 'static,
{
    type Output = Out;

    fn listen(&self, event: &In) -> Option<Out> {
        (self.mapper)(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct InputEvent {
        content: String,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct CommandEvent {
        command: String,
    }

    #[test]
    fn test_optional_map_transforms() {
        let parser = OptionalMapListener::new(|e: &InputEvent| {
            if e.content.starts_with("!") {
                Some(CommandEvent {
                    command: e.content[1..].to_string(),
                })
            } else {
                None
            }
        });

        let input = InputEvent {
            content: "!ping".into(),
        };
        let result = parser.listen(&input);
        assert_eq!(
            result,
            Some(CommandEvent {
                command: "ping".into()
            })
        );
    }

    #[test]
    fn test_optional_map_filters() {
        let parser = OptionalMapListener::new(|e: &InputEvent| {
            if e.content.starts_with("!") {
                Some(CommandEvent {
                    command: e.content[1..].to_string(),
                })
            } else {
                None
            }
        });

        let input = InputEvent {
            content: "hello".into(),
        };
        let result = parser.listen(&input);
        assert_eq!(result, None);
    }

    #[test]
    fn test_optional_map_edge_cases() {
        let parser = OptionalMapListener::new(|e: &InputEvent| {
            e.content.strip_prefix("!").map(|cmd| CommandEvent {
                command: cmd.into(),
            })
        });

        // Just "!" prefix
        let input = InputEvent {
            content: "!".into(),
        };
        assert_eq!(
            parser.listen(&input),
            Some(CommandEvent { command: "".into() })
        );

        // Empty string
        let input = InputEvent { content: "".into() };
        assert_eq!(parser.listen(&input), None);
    }
}
