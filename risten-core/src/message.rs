//! Message trait for event types.

/// A marker trait for events and triggers within the system.
///
/// Messages must be `Send + Sync + 'static` to be safe for async use.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid Message",
    label = "must be `Send + Sync + 'static`",
    note = "All events in Risten must be thread-safe and static."
)]
pub trait Message: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> Message for T {}
