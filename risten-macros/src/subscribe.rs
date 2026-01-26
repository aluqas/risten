//! Subscribe-related macros.
//!
//! This module contains:
//! - `#[subscribe]` - Attribute macro for subscribing functions to event handlers
//! - `#[on]` - Alias for `#[subscribe]`

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ItemFn, LitInt, Token, Type, parse::Parse, parse_macro_input};

/// Arguments for the `#[subscribe]` macro.
pub(crate) struct SubscribeArgs {
    /// Optional explicit event type.
    pub event_type: Option<Type>,
    /// Priority for handler execution (higher = earlier).
    pub priority: i32,
}

impl Parse for SubscribeArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut event_type = None;
        let mut priority = 0;

        // Check if we have named arguments or just a type
        if input.is_empty() {
            return Ok(SubscribeArgs {
                event_type: None,
                priority: 0,
            });
        }

        // Try to parse as just a type first
        if input.peek(Ident) && !input.peek2(Token![=]) {
            // This looks like a type, not a named arg
            event_type = Some(input.parse()?);
        }

        // Parse any remaining named arguments
        while !input.is_empty() {
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }

            if input.is_empty() {
                break;
            }

            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "priority" => {
                    let lit: LitInt = input.parse()?;
                    priority = lit.base10_parse()?;
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown attribute: {}", other),
                    ));
                }
            }
        }

        Ok(SubscribeArgs {
            event_type,
            priority,
        })
    }
}

/// Generates a handler that wraps user function to return `Result<(), ExtractError>`.
pub(crate) fn generate_subscribe_handler_impl(
    input: &ItemFn,
    event_type: Option<&Type>,
) -> (proc_macro2::TokenStream, Type) {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let is_async = input.sig.asyncness.is_some();
    let inputs = &input.sig.inputs;
    let arg_count = inputs.len();

    let struct_name = fn_name.clone();

    // Determine event type from first argument or explicit type
    let inferred_event_type = match event_type {
        Some(ty) => quote! { #ty },
        None => match inputs.first() {
            Some(FnArg::Typed(pat_type)) => {
                let ty = &pat_type.ty;
                quote! { #ty }
            }
            _ => panic!("subscribe function must have at least one argument or specify event type"),
        },
    };

    let parsed_event_type: Type = syn::parse2(inferred_event_type.clone()).unwrap();

    // For single-argument handlers, use simpler code path
    if arg_count == 1 && event_type.is_none() {
        let (input_pat, input_type) = match inputs.first().unwrap() {
            FnArg::Typed(pat_type) => (&pat_type.pat, &pat_type.ty),
            _ => panic!("subscribe function must take at least one argument"),
        };

        let call_body = if is_async {
            quote! {
                #fn_block
                ::core::result::Result::Ok(())
            }
        } else {
            quote! {
                { #fn_block }
                ::core::result::Result::Ok(())
            }
        };

        let impl_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy, Debug, Default)]
            #[doc = concat!("Auto-generated Handler from `#[risten::subscribe]` on `", stringify!(#fn_name), "`")]
            #fn_vis struct #struct_name;

            impl ::risten::Handler<#input_type> for #struct_name {
                type Output = ::core::result::Result<(), ::risten::ExtractError>;

                async fn call(&self, #input_pat: #input_type) -> Self::Output {
                    #call_body
                }
            }
        };

        return (impl_code, *input_type.clone());
    }

    // Multi-argument handlers need extraction
    let mut arg_pats = Vec::new();
    let mut arg_types = Vec::new();
    let mut extraction_code = Vec::new();

    for (i, arg) in inputs.iter().enumerate() {
        match arg {
            FnArg::Typed(pat_type) => {
                let pat = &pat_type.pat;
                let ty = &pat_type.ty;
                let arg_name = Ident::new(&format!("__arg_{}", i), fn_name.span());

                arg_pats.push(quote! { #pat });
                arg_types.push(quote! { #ty });

                extraction_code.push(quote! {
                    let #arg_name: #ty = <#ty as ::risten::AsyncFromEvent<_>>::from_event(&__event)
                        .await
                        .map_err(|e| ::risten::ExtractError::new(e.to_string()))?;
                });
            }
            FnArg::Receiver(_) => panic!("subscribe handler cannot have self parameter"),
        }
    }

    let arg_names: Vec<_> = (0..arg_count)
        .map(|i| Ident::new(&format!("__arg_{}", i), fn_name.span()))
        .collect();

    let inner_call = if is_async {
        quote! {
            async fn __inner(#(#arg_pats: #arg_types),*) {
                #fn_block
            }
            __inner(#(#arg_names),*).await;
            ::core::result::Result::Ok(())
        }
    } else {
        quote! {
            fn __inner(#(#arg_pats: #arg_types),*) {
                #fn_block
            }
            __inner(#(#arg_names),*);
            ::core::result::Result::Ok(())
        }
    };

    let impl_code = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default)]
        #[doc = concat!("Auto-generated Handler from `#[risten::subscribe]` on `", stringify!(#fn_name), "`")]
        #fn_vis struct #struct_name;

        impl ::risten::Handler<#inferred_event_type> for #struct_name {
            type Output = ::core::result::Result<(), ::risten::ExtractError>;

            async fn call(&self, __event: #inferred_event_type) -> Self::Output {
                #(#extraction_code)*
                #inner_call
            }
        }
    };

    (impl_code, parsed_event_type)
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
pub fn subscribe_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as SubscribeArgs);
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let priority = args.priority;

    // Validate: must be async
    if input.sig.asyncness.is_none() {
        return syn::Error::new_spanned(&input.sig.fn_token, "subscribe handler must be async")
            .to_compile_error()
            .into();
    }

    // Validate: must have at least one argument
    if input.sig.inputs.is_empty() {
        return syn::Error::new_spanned(
            &input.sig.inputs,
            "subscribe handler must have at least one argument (the event)",
        )
        .to_compile_error()
        .into();
    }

    let (handler_impl, event_type) =
        generate_subscribe_handler_impl(&input, args.event_type.as_ref());
    let handler_struct_name = fn_name;

    let static_name = Ident::new(
        &format!("__HANDLER_INSTANCE_{}", fn_name).to_uppercase(),
        fn_name.span(),
    );
    let wrapper_name = Ident::new(
        &format!("__HANDLER_WRAPPER_{}", fn_name).to_uppercase(),
        fn_name.span(),
    );

    let submit_code = quote! {
        #[allow(non_upper_case_globals)]
        static #static_name: #handler_struct_name = #handler_struct_name;

        #[allow(non_upper_case_globals)]
        static #wrapper_name: ::risten::routing::ErasedHandlerWrapper<#event_type, #handler_struct_name> =
            ::risten::routing::ErasedHandlerWrapper::new(#handler_struct_name);

        ::risten::inventory::submit! {
            ::risten::routing::HandlerRegistration {
                type_id: ::std::any::TypeId::of::<#event_type>(),
                handler: &#wrapper_name,
                priority: #priority,
            }
        }
    };

    let expanded = quote! {
        #handler_impl
        #submit_code
    };

    TokenStream::from(expanded)
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
pub fn on_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    subscribe_impl(attr, item)
}
