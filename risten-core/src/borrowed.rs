//! Zero-Copy message and listener support.

use crate::{listener::Listener, message::Message};

/// A marker trait for borrowed messages.
pub trait RawMessage<'a>: Send + Sync {}

// Any Send + Sync type with the lifetime is a RawMessage
impl<'a, T: Send + Sync + ?Sized> RawMessage<'a> for T {}

/// A listener that can produce borrowed output from borrowed input (GAT).
pub trait BorrowedListener<In>: Send + Sync + 'static {
    /// The output type, which may borrow from the input.
    type Output<'a>: RawMessage<'a>
    where
        In: 'a;

    /// Inspect and optionally transform the input.
    fn listen<'a>(&self, event: &'a In) -> Option<Self::Output<'a>>;
}

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

/// Chain of two borrowed listeners.
pub struct BorrowedChain<A, B> {
    pub first: A,
    pub second: B,
}

impl<A, B> BorrowedChain<A, B> {
    /// Create a new chain from two borrowed listeners.
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}
