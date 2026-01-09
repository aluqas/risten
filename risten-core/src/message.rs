//! Message trait for event types.

/// A marker trait for events and triggers within the system.
///
/// Messages must be `Send + Sync + 'static` to be safe for async use.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Clone)]
/// struct MyEvent { id: u64 }
///
/// impl Message for MyEvent {}
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid Message",
    label = "must be `Send + Sync + 'static`",
    note = "All events in Risten must be thread-safe and static."
)]
pub trait Message: Send + Sync + 'static {}

// Common Message implementations
impl Message for () {}
impl Message for String {}
impl Message for &'static str {}
impl<T: Message> Message for Box<T> {}
impl<T: Message> Message for std::sync::Arc<T> {}
impl<T: Message> Message for Vec<T> {}
impl<T: Message> Message for Option<T> {}
impl<T: Message, E: Message> Message for Result<T, E> {}
