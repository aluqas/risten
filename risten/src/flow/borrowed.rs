//! Zero-Copy message and listener support using Generic Associated Types (GAT).
//!
//! This module provides borrowed-data versions of the core traits for
//! zero-copy event processing. Use these when performance is critical
//! and you want to avoid cloning event data.
//!
//! # Overview
//!
//! The standard `Message` and `Listener` traits require `'static` lifetimes,
//! meaning data must be owned or cloned. The borrowed variants allow
//! processing references directly:
//!
//! | Standard | Borrowed | Use Case |
//! |:---------|:---------|:---------|
//! | `Message` | `RawMessage<'a>` | Borrowed event data |
//! | `Listener<In>` | `BorrowedListener<In>` | Zero-copy transformation |
//!
//! # Example
//!
//! ```rust,ignore
//! use risten::borrowed::{BorrowedListener, RawMessage};
//!
//! struct ContentExtractor;
//!
//! impl BorrowedListener<DiscordMessage> for ContentExtractor {
//!     type Output<'a> = &'a str where DiscordMessage: 'a;
//!
//!     fn listen<'a>(&self, event: &'a DiscordMessage) -> Option<&'a str> {
//!         Some(&event.content)  // Zero-copy!
//!     }
//! }
//! ```

/// A marker trait for borrowed messages with lifetime.
///
/// Unlike `Message`, this trait allows non-`'static` data,
/// enabling zero-copy event processing.
///
/// # Blanket Implementations
///
/// - All `Message` types are also `RawMessage<'a>` for any `'a`
/// - All `Send + Sync` types are `RawMessage<'a>`
pub trait RawMessage<'a>: Send + Sync {}

// Any Send + Sync type with the lifetime is a RawMessage
impl<'a, T: Send + Sync + ?Sized> RawMessage<'a> for T {}

/// A listener that can produce borrowed output from borrowed input.
///
/// This is the zero-copy counterpart to [`Listener`](crate::Listener).
/// It uses Generic Associated Types (GAT) to express that the output
/// lifetime depends on the input lifetime.
///
/// # When to Use
///
/// Use `BorrowedListener` when:
/// - You want to extract references from events without cloning
/// - You're parsing/slicing string content
/// - You're extracting field references
///
/// Use standard `Listener` when:
/// - You need to transform data (not just reference it)
/// - Your output must outlive the input
/// - You're building structs with owned data
///
/// # Example
///
/// ```rust,ignore
/// use risten::borrowed::BorrowedListener;
///
/// // Extract a slice from the content
/// struct PrefixExtractor {
///     prefix: String,
/// }
///
/// impl BorrowedListener<ChatMessage> for PrefixExtractor {
///     type Output<'a> = &'a str where ChatMessage: 'a;
///
///     fn listen<'a>(&self, event: &'a ChatMessage) -> Option<&'a str> {
///         event.content.strip_prefix(&self.prefix)
///     }
/// }
/// ```
pub trait BorrowedListener<In>: Send + Sync + 'static {
    /// The output type, which may borrow from the input.
    ///
    /// The lifetime `'a` is bound to the input, allowing
    /// the output to reference the input's data.
    type Output<'a>: RawMessage<'a>
    where
        In: 'a;

    /// Inspect and optionally transform the input.
    ///
    /// Returns `None` if this listener does not handle the input.
    fn listen<'a>(&self, event: &'a In) -> Option<Self::Output<'a>>;
}

// ============================================================================
// Blanket Implementation: Standard Listener -> BorrowedListener
// ============================================================================

use crate::{core::message::Message, flow::listener::Listener};

/// Blanket implementation allowing any `Listener` to be used as a `BorrowedListener`.
///
/// The output lifetime is independent of the input (it's `'static`),
/// so any standard `Listener` is trivially a `BorrowedListener`.
impl<L, In> BorrowedListener<In> for L
where
    L: Listener<In>,
    In: Message,
    L::Output: Message,
{
    type Output<'a>
        = L::Output
    where
        In: 'a;

    fn listen<'a>(&self, event: &'a In) -> Option<L::Output> {
        Listener::listen(self, event)
    }
}

// ============================================================================
// Chaining for BorrowedListener
// ============================================================================

/// Chain of two borrowed listeners.
pub struct BorrowedChain<A, B> {
    first: A,
    second: B,
}

impl<A, B> BorrowedChain<A, B> {
    /// Create a new chain from two borrowed listeners.
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

// Note: Chaining BorrowedListeners is complex due to lifetime dependencies.
// The output of the first listener becomes the input of the second,
// but the second's output may borrow from the first's output.
// This requires higher-kinded types or complex trait bounds.
//
// For now, we provide the struct but not a full BorrowedListener impl.
// Users can manually chain by calling listen() sequentially.

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestMessage {
        content: String,
    }

    // Test that standard Message types satisfy RawMessage
    #[test]
    fn test_message_is_raw_message() {
        fn assert_raw_message<'a, T: RawMessage<'a>>() {}
        assert_raw_message::<TestMessage>();
        assert_raw_message::<String>();
        assert_raw_message::<&str>();
    }

    // A borrowed listener that extracts a reference
    struct ContentRef;

    impl BorrowedListener<TestMessage> for ContentRef {
        type Output<'a>
            = &'a str
        where
            TestMessage: 'a;

        fn listen<'a>(&self, event: &'a TestMessage) -> Option<&'a str> {
            Some(&event.content)
        }
    }

    #[test]
    fn test_borrowed_listener_extracts_ref() {
        let listener = ContentRef;
        let msg = TestMessage {
            content: "hello world".to_string(),
        };

        let result = listener.listen(&msg);
        assert_eq!(result, Some("hello world"));
    }

    // A borrowed listener that conditionally extracts
    struct PrefixStripper {
        prefix: &'static str,
    }

    impl BorrowedListener<TestMessage> for PrefixStripper {
        type Output<'a>
            = &'a str
        where
            TestMessage: 'a;

        fn listen<'a>(&self, event: &'a TestMessage) -> Option<&'a str> {
            event.content.strip_prefix(self.prefix)
        }
    }

    #[test]
    fn test_prefix_stripper() {
        let listener = PrefixStripper { prefix: "!" };

        let msg1 = TestMessage {
            content: "!hello".to_string(),
        };
        let msg2 = TestMessage {
            content: "hello".to_string(),
        };

        assert_eq!(listener.listen(&msg1), Some("hello"));
        assert_eq!(listener.listen(&msg2), None);
    }

    // Test blanket impl: standard Listener as BorrowedListener
    struct OwnedExtractor;

    impl Listener<TestMessage> for OwnedExtractor {
        type Output = String;

        fn listen(&self, event: &TestMessage) -> Option<String> {
            Some(event.content.to_uppercase())
        }
    }

    #[test]
    fn test_listener_as_borrowed_listener() {
        let listener = OwnedExtractor;
        let msg = TestMessage {
            content: "hello".to_string(),
        };

        // Use via BorrowedListener trait
        let result: Option<String> = BorrowedListener::listen(&listener, &msg);
        assert_eq!(result, Some("HELLO".to_string()));
    }
}
