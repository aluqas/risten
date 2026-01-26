//! Router and dispatch related macros.
//!
//! This module contains:
//! - `#[dispatch]` - Attribute macro for creating dispatch implementations from enums

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Meta, parse_macro_input};

/// Extracts handler path from doc comments.
///
/// Looks for `@handler(HandlerPath)` in doc comments and parses the handler path.
pub(crate) fn extract_handler_attr(attrs: &[Attribute]) -> Option<syn::Path> {
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let content = lit_str.value();
                        if let Some(start) = content.find("@handler(") {
                            let after = &content[start + 9..];
                            if let Some(end) = after.find(')') {
                                let handler_name = after[..end].trim();
                                if let Ok(path) = syn::parse_str::<syn::Path>(handler_name) {
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Implementation of the `#[dispatch]` macro.
///
/// Creates dispatch implementations for enum types, generating:
/// - `dispatch_match()` - Returns `HookResult` based on variant
/// - `variant_name()` - Returns the variant name as a string
/// - `dispatch_to_hooks()` - Async dispatch to registered hooks
///
/// # Example
///
/// ```rust,ignore
/// #[risten::dispatch]
/// enum MyEvent {
///     /// @handler(my_handler)
///     Message(MessageData),
///     Disconnect,
/// }
/// ```
pub fn dispatch_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let enum_name = &input.ident;
    let _vis = &input.vis;

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => {
            return syn::Error::new_spanned(&input, "#[dispatch] can only be used on enums")
                .to_compile_error()
                .into();
        }
    };

    let match_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        match &variant.fields {
            syn::Fields::Unnamed(_) => {
                quote! {
                    #enum_name::#variant_name(_inner) => {
                        ::risten::HookResult::Next
                    }
                }
            }
            syn::Fields::Unit => {
                quote! {
                    #enum_name::#variant_name => {
                        ::risten::HookResult::Next
                    }
                }
            }
            _ => {
                quote! {
                    #enum_name::#variant_name { .. } => {
                        ::risten::HookResult::Next
                    }
                }
            }
        }
    });

    let name_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        match &variant.fields {
            syn::Fields::Unnamed(_) => {
                quote! {
                    #enum_name::#variant_name(_) => stringify!(#variant_name)
                }
            }
            syn::Fields::Unit => {
                quote! {
                    #enum_name::#variant_name => stringify!(#variant_name)
                }
            }
            _ => {
                quote! {
                    #enum_name::#variant_name { .. } => stringify!(#variant_name)
                }
            }
        }
    });

    let static_dispatch_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let handler_path = extract_handler_attr(&variant.attrs);

        match &variant.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                if let Some(handler) = handler_path {
                    quote! {
                        #enum_name::#variant_name(inner) => {
                            let hook = #handler;
                            ::risten::Hook::on_event(&hook, inner).await
                        }
                    }
                } else {
                    quote! {
                        #enum_name::#variant_name(_) => {
                            ::core::result::Result::Ok(::risten::HookResult::Next)
                        }
                    }
                }
            }
            syn::Fields::Unit => {
                quote! {
                    #enum_name::#variant_name => {
                        ::core::result::Result::Ok(::risten::HookResult::Next)
                    }
                }
            }
            _ => {
                quote! {
                    #enum_name::#variant_name { .. } => {
                        ::core::result::Result::Ok(::risten::HookResult::Next)
                    }
                }
            }
        }
    });

    let expanded = quote! {
        #input

        impl #enum_name {
            /// Dispatches to the appropriate handler based on the variant.
            pub fn dispatch_match(&self) -> ::risten::HookResult {
                match self {
                    #(#match_arms),*
                }
            }

            /// Returns the name of the current variant as a static string.
            pub fn variant_name(&self) -> &'static str {
                match self {
                    #(#name_arms),*
                }
            }

            /// Asynchronously dispatches to the registered hooks.
            pub async fn dispatch_to_hooks(&self) -> ::core::result::Result<::risten::HookResult, ::std::boxed::Box<dyn ::std::error::Error + Send + Sync>> {
                match self {
                    #(#static_dispatch_arms),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
