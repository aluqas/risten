//! Risten procedural macros.
//!
//! This crate provides procedural macros for the Risten event framework:
//!
//! - `#[derive(Message)]` - Derive macro for implementing the `Message` trait
//! - `#[event]` - Create Hook implementations from async functions
//! - `#[handler]` - Create Handler implementations with extraction support
//! - `#[subscribe]` / `#[on]` - Register handlers with the global dispatcher
//! - `#[main]` - Wrap main function with tokio runtime
//! - `#[dispatch]` - Create dispatch implementations for enum types

mod event;
mod handler;
mod main_fn;
mod router_macro;
mod subscribe;

use proc_macro::TokenStream;

/// Derive macro for implementing `Message` trait.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Message)]
/// struct MyEvent {
///     data: String,
/// }
/// ```
#[proc_macro_derive(Message)]
pub fn derive_message(input: TokenStream) -> TokenStream {
    event::derive_message_impl(input)
}

/// Attribute macro for creating Hook implementations from async functions.
///
/// # Arguments
///
/// - `priority` - Optional priority value (higher runs first)
/// - `name` - Optional custom struct name
/// - `filter` - Optional filter expression
///
/// # Example
///
/// ```rust,ignore
/// #[risten::event(priority = 10)]
/// async fn my_hook(event: &MyEvent) -> Result<HookResult, Box<dyn Error + Send + Sync>> {
///     println!("Handling event: {:?}", event);
///     Ok(HookResult::Next)
/// }
/// ```
#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
    event::event_impl(attr, item)
}

/// Attribute macro for creating Handler implementations with extraction support.
///
/// # Arguments
///
/// - `name` - Optional custom struct name
/// - `priority` - Optional priority value
///
/// # Example
///
/// ```rust,ignore
/// #[risten::handler]
/// async fn handle_message(event: &MessageEvent) -> Result<(), Box<dyn Error + Send + Sync>> {
///     println!("Message: {:?}", event);
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    handler::handler_impl(attr, item)
}

/// Subscribe a function to handle events of a specific type.
///
/// This macro registers the function with the global handler registry,
/// allowing it to be automatically discovered and executed by `DispatchRouter`.
///
/// # Usage
///
/// ```rust,ignore
/// // Simple handler - event type inferred from first argument
/// #[risten::subscribe]
/// async fn on_message(event: MessageEvent) {
///     println!("Received: {:?}", event);
/// }
///
/// // With explicit event type
/// #[risten::subscribe(MyEvent)]
/// async fn on_my_event(event: MyEvent) {
///     // ...
/// }
///
/// // With priority (higher = earlier execution)
/// #[risten::subscribe(priority = 10)]
/// async fn high_priority_handler(event: MessageEvent) {
///     // ...
/// }
///
/// // With multiple extractors
/// #[risten::subscribe]
/// async fn with_context(event: MessageEvent, user: UserContext) {
///     // user is extracted via AsyncFromEvent
/// }
/// ```
#[proc_macro_attribute]
pub fn subscribe(attr: TokenStream, item: TokenStream) -> TokenStream {
    subscribe::subscribe_impl(attr, item)
}

/// Alias for `#[subscribe]`.
///
/// This macro is identical to `#[subscribe]` and can be used interchangeably.
/// Some developers prefer `#[on]` for its brevity.
///
/// # Example
///
/// ```rust,ignore
/// #[risten::on]
/// async fn on_message(event: MessageEvent) {
///     println!("Received: {:?}", event);
/// }
/// ```
#[proc_macro_attribute]
pub fn on(attr: TokenStream, item: TokenStream) -> TokenStream {
    subscribe::on_impl(attr, item)
}

/// Wraps the main function with `#[tokio::main]` for async runtime support.
///
/// # Example
///
/// ```rust,ignore
/// #[risten::main]
/// async fn main() {
///     // Your async code here
/// }
/// ```
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    main_fn::main_impl(attr, item)
}

/// Creates dispatch implementations for enum types.
///
/// Generates the following methods:
/// - `dispatch_match()` - Returns `HookResult` based on variant
/// - `variant_name()` - Returns the variant name as a string
/// - `dispatch_to_hooks()` - Async dispatch to registered hooks
///
/// # Handler Registration
///
/// Use `@handler(HandlerPath)` in doc comments to register handlers:
///
/// ```rust,ignore
/// #[risten::dispatch]
/// enum MyEvent {
///     /// @handler(my_handler)
///     Message(MessageData),
///     Disconnect,
/// }
/// ```
#[proc_macro_attribute]
pub fn dispatch(attr: TokenStream, item: TokenStream) -> TokenStream {
    router_macro::dispatch_impl(attr, item)
}
