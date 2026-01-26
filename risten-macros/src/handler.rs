//! Handler-related macros.
//!
//! This module contains:
//! - `#[handler]` - Attribute macro for creating Handler implementations with extraction

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ItemFn, LitInt, LitStr, Token, Type, parse::Parse, parse_macro_input};

/// Arguments for the `#[handler]` macro.
pub(crate) struct HandlerArgs {
    pub name: Option<String>,
    pub priority: Option<i32>,
}

impl Parse for HandlerArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut priority = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "priority" => {
                    let lit: LitInt = input.parse()?;
                    priority = Some(lit.base10_parse()?);
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown attribute: {}", other),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(HandlerArgs { name, priority })
    }
}

/// Implementation of the `#[handler]` macro.
pub fn handler_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as HandlerArgs);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;

    if input.sig.asyncness.is_none() {
        return syn::Error::new_spanned(&input.sig.fn_token, "Handler function must be async")
            .to_compile_error()
            .into();
    }

    let struct_name = if let Some(ref custom_name) = args.name {
        Ident::new(custom_name, fn_name.span())
    } else {
        fn_name.clone()
    };

    let inputs = &input.sig.inputs;
    if inputs.is_empty() {
        return syn::Error::new_spanned(
            inputs,
            "Handler function must have at least one argument (the event)",
        )
        .to_compile_error()
        .into();
    }

    let event_type = match inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            if let Type::Reference(type_ref) = &*pat_type.ty {
                &type_ref.elem
            } else {
                return syn::Error::new_spanned(
                    &pat_type.ty,
                    "Handler event argument must be a reference (&Event)",
                )
                .to_compile_error()
                .into();
            }
        }
        _ => {
            return syn::Error::new_spanned(
                inputs,
                "Handler function must take an event argument: fn(event: &Event, ...)",
            )
            .to_compile_error()
            .into();
        }
    };

    let _arg_pats: Vec<_> = inputs.iter().collect();
    let _arg_bindings: Vec<_> = inputs
        .iter()
        .enumerate()
        .map(|(i, arg)| {
            if let FnArg::Typed(pat_type) = arg {
                let pat = &pat_type.pat;
                quote! { #pat }
            } else {
                let ident = Ident::new(&format!("__arg{}", i), proc_macro2::Span::call_site());
                quote! { #ident }
            }
        })
        .collect();

    let extractor_bindings: Vec<_> = inputs
        .iter()
        .skip(1)
        .enumerate()
        .filter_map(|(i, arg)| {
            if let FnArg::Typed(pat_type) = arg {
                let pat = &pat_type.pat;
                let ty = &pat_type.ty;
                let _idx = i + 1;
                Some(quote! {
                    let #pat: #ty = match <#ty as ::risten::FromEvent<#event_type>>::from_event(__event) {
                        ::core::result::Result::Ok(v) => v,
                        ::core::result::Result::Err(e) => {
                            return ::core::result::Result::Err(
                                ::std::boxed::Box::new(e) as ::std::boxed::Box<dyn ::std::error::Error + Send + Sync>
                            );
                        }
                    };
                })
            } else {
                None
            }
        })
        .collect();

    let first_arg = if let Some(FnArg::Typed(pat_type)) = inputs.first() {
        let pat = &pat_type.pat;
        quote! { let #pat = __event; }
    } else {
        quote! {}
    };

    let priority_impl = args.priority.map(|p| {
        quote! {
            impl #struct_name {
                /// The priority of this handler.
                pub const PRIORITY: i32 = #p;
            }
        }
    });

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default)]
        #[doc = concat!("Auto-generated Handler from `#[risten::handler]` on `", stringify!(#fn_name), "`")]
        #fn_vis struct #struct_name;

        #priority_impl

        impl ::risten::Handler<#event_type> for #struct_name {
            async fn handle(
                &self,
                __event: &#event_type,
            ) -> ::core::result::Result<(), ::std::boxed::Box<dyn ::std::error::Error + Send + Sync>> {
                #first_arg
                #(#extractor_bindings)*
                #fn_block
                ::core::result::Result::Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}
