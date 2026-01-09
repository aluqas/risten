use crate::core::message::Message;

/// A listener sits at the entry or intermediate points of the event pipeline (Phase 1).
///
/// Its role is to inspect (borrow) an input event and optionally produce an output event (Trigger)
/// or decide to route it further. It is synchronous and lightweight.
///
/// # Example
///
/// ```rust
/// use risten::{Listener, Message};
///
/// #[derive(Clone)]
/// struct ChatMessage { content: String, author: String }
///
/// #[derive(Clone)]
/// struct CommandTrigger { command: String, author: String }
///
/// struct CommandPrefixListener {
///     prefix: String,
/// }
///
/// impl Listener<ChatMessage> for CommandPrefixListener {
///     type Output = CommandTrigger;
///
///     fn listen(&self, event: &ChatMessage) -> Option<Self::Output> {
///         if event.content.starts_with(&self.prefix) {
///             Some(CommandTrigger {
///                 command: event.content[self.prefix.len()..].to_string(),
///                 author: event.author.clone(),
///             })
///         } else {
///             None
///         }
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Listener` for `{In}`",
    label = "missing `Listener` implementation",
    note = "Listeners must implement the `listen` method to process `{In}`."
)]
pub trait Listener<In: Message>: Send + Sync + 'static {
    /// The type of message this listener produces to be passed to the next stage.
    type Output: Message;

    /// Inspects the input event and transforms it into the Output type if applicable.
    ///
    /// Returns `None` if the event should be ignored by this listener pipeline.
    fn listen(&self, event: &In) -> Option<Self::Output>;

    /// Chains this listener with another listener.
    fn and_then<Next>(self, next: Next) -> Chain<Self, Next>
    where
        Self: Sized,
        Next: Listener<Self::Output>,
    {
        Chain {
            first: self,
            second: next,
        }
    }

    /// Connects this listener to a handler, creating a complete pipeline.
    ///
    /// The handler receives the listener's output and performs async work.
    fn handler<H>(self, handler: H) -> Pipeline<Self, H>
    where
        Self: Sized,
        H: crate::flow::handler::Handler<Self::Output>,
    {
        Pipeline {
            listener: self,
            handler,
        }
    }

    /// Connects this listener to a handler (alias for consistency with older API).
    #[deprecated(note = "use `handler` instead")]
    fn endpoint<H>(self, handler: H) -> Pipeline<Self, H>
    where
        Self: Sized,
        H: crate::flow::handler::Handler<Self::Output>,
    {
        self.handler(handler)
    }
}

/// A chain of two listeners.
pub struct Chain<A, B> {
    first: A,
    second: B,
}

impl<A, B, In> Listener<In> for Chain<A, B>
where
    In: Message,
    A: Listener<In>,
    B: Listener<A::Output>,
{
    type Output = B::Output;

    fn listen(&self, event: &In) -> Option<Self::Output> {
        let intermediate = self.first.listen(event)?;
        self.second.listen(&intermediate)
    }
}

/// A complete pipeline connecting a Listener to a Handler.
///
/// This struct holds the logic to process an event from start to finish.
/// Implements `Hook` so it can be registered with a dispatcher.
///
/// # Example
///
/// ```rust
/// use risten::{Listener, Handler};
///
/// # #[derive(Clone)] struct Event { msg: String }
/// # #[derive(Clone)] struct Trigger { data: String }
/// # struct MyListener;
/// # impl Listener<Event> for MyListener {
/// #     type Output = Trigger;
/// #     fn listen(&self, e: &Event) -> Option<Trigger> { Some(Trigger { data: e.msg.clone() }) }
/// # }
/// # struct MyHandler;
/// # impl Handler<Trigger> for MyHandler {
/// #     type Output = ();
/// #     async fn call(&self, _: Trigger) {}
/// # }
///
/// let listener = MyListener;
/// let handler = MyHandler;
/// let pipeline = listener.handler(handler);
/// // pipeline now implements Hook<Event>
/// ```
pub struct Pipeline<L, H> {
    pub(crate) listener: L,
    pub(crate) handler: H,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct StringEvent(String);

    #[derive(Clone, Debug, PartialEq)]
    struct IntEvent(i32);

    struct ToIntListener;
    impl Listener<StringEvent> for ToIntListener {
        type Output = IntEvent;
        fn listen(&self, event: &StringEvent) -> Option<Self::Output> {
            event.0.parse().ok().map(IntEvent)
        }
    }

    struct DoubleListener;
    impl Listener<IntEvent> for DoubleListener {
        type Output = IntEvent;
        fn listen(&self, event: &IntEvent) -> Option<Self::Output> {
            Some(IntEvent(event.0 * 2))
        }
    }

    #[test]
    fn test_and_then_chaining() {
        let chain = ToIntListener.and_then(DoubleListener);

        // "10" -> 10 -> 20
        let event = StringEvent("10".to_string());
        let result = chain.listen(&event);
        assert_eq!(result, Some(IntEvent(20)));

        // "abc" -> None -> None
        let event = StringEvent("abc".to_string());
        let result = chain.listen(&event);
        assert_eq!(result, None);
    }
}
