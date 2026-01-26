//! Main function macro.
//!
//! This module contains:
//! - `#[main]` - Attribute macro for wrapping main functions with tokio runtime

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

/// Implementation of the `#[main]` macro.
///
/// This macro wraps the function with `#[tokio::main]` to enable async runtime.
///
/// # Example
///
/// ```rust,ignore
/// #[risten::main]
/// async fn main() {
///     // Your async code here
/// }
/// ```
pub fn main_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let expanded = quote! {
        #[::tokio::main]
        #input
    };
    TokenStream::from(expanded)
}
