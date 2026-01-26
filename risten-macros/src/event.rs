//! Event-related macros.
//!
//! This module contains:
//! - `#[derive(Message)]` - Derive macro for implementing the `Message` trait
//! - `#[event]` - Attribute macro for creating Hook implementations from functions

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, Expr, FnArg, Ident, ItemFn, LitInt, LitStr, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Derive macro for implementing `Message` trait.
pub fn derive_message_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::risten::Message for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

/// Arguments for the `#[event]` macro.
pub(crate) struct EventArgs {
    pub priority: Option<i32>,
    pub name: Option<String>,
    pub filter: Option<Expr>,
}

impl Parse for EventArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut priority = None;
        let mut name = None;
        let mut filter = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "priority" => {
                    let lit: LitInt = input.parse()?;
                    priority = Some(lit.base10_parse()?);
                }
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "filter" => {
                    let expr: Expr = input.parse()?;
                    filter = Some(expr);
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

        Ok(EventArgs {
            priority,
            name,
            filter,
        })
    }
}

/// Implementation of the `#[event]` attribute macro.
pub fn event_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as EventArgs);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;

    if input.sig.asyncness.is_none() {
        return syn::Error::new_spanned(&input.sig.fn_token, "Hook function must be async")
            .to_compile_error()
            .into();
    }

    let inputs = &input.sig.inputs;
    let (event_pat, event_type) = match inputs.first() {
        Some(FnArg::Typed(pat_type)) => {
            if let Type::Reference(type_ref) = &*pat_type.ty {
                (&pat_type.pat, &type_ref.elem)
            } else {
                return syn::Error::new_spanned(
                    &pat_type.ty,
                    "Hook event argument must be a reference (&Event)",
                )
                .to_compile_error()
                .into();
            }
        }
        _ => {
            return syn::Error::new_spanned(
                inputs,
                "Hook function must take an event argument: fn(event: &Event)",
            )
            .to_compile_error()
            .into();
        }
    };

    let struct_name = if let Some(ref custom_name) = args.name {
        Ident::new(custom_name, fn_name.span())
    } else {
        fn_name.clone()
    };

    let priority_impl = args.priority.map(|p| {
        quote! {
            impl #struct_name {
                /// The priority of this hook. Higher values run first.
                pub const PRIORITY: i32 = #p;
            }
        }
    });

    let filter_check = args.filter.as_ref().map(|filter_expr| {
        quote! {
            if !(#filter_expr)(#event_pat) {
                return ::core::result::Result::Ok(::risten::HookResult::Next);
            }
        }
    });

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default)]
        #[doc = concat!("Auto-generated Hook from `#[risten::event]` on `", stringify!(#fn_name), "`")]
        #fn_vis struct #struct_name;

        #priority_impl

        impl ::risten::Hook<#event_type> for #struct_name {
            async fn on_event(
                &self,
                #event_pat: &#event_type,
            ) -> ::core::result::Result<::risten::HookResult, ::std::boxed::Box<dyn ::std::error::Error + Send + Sync>> {
                #filter_check
                #fn_block
            }
        }
    };

    TokenStream::from(expanded)
}
