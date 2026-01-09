use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, FnArg, Ident, ItemFn, LitInt, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Parsed attributes for #[risten::event(...)]
struct EventArgs {
    priority: Option<i32>,
    name: Option<String>,
}

impl Parse for EventArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut priority = None;
        let mut name = None;

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

        Ok(EventArgs { priority, name })
    }
}

/// Attribute macro to convert async functions into Hook implementations.
#[proc_macro_attribute]
pub fn event(attr: TokenStream, item: TokenStream) -> TokenStream {
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
                #fn_block
            }
        }
    };

    TokenStream::from(expanded)
}

/// Parsed attributes for #[risten::handler(...)]
struct HandlerArgs {
    name: Option<String>,
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
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

        Ok(HandlerArgs { name })
    }
}

/// Attribute macro to convert async functions into Handler implementations.
#[proc_macro_attribute]
pub fn handler(attr: TokenStream, item: TokenStream) -> TokenStream {
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

    let inputs = &input.sig.inputs;
    let (input_pat, input_type) = match inputs.first() {
        Some(FnArg::Typed(pat_type)) => (&pat_type.pat, &pat_type.ty),
        _ => {
            return syn::Error::new_spanned(
                inputs,
                "Handler function must take exactly one argument",
            )
            .to_compile_error()
            .into();
        }
    };

    if inputs.len() > 1 {
        return syn::Error::new_spanned(inputs, "Handler function must take exactly one argument")
            .to_compile_error()
            .into();
    }

    let output_type = match &input.sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };

    let struct_name = if let Some(ref custom_name) = args.name {
        Ident::new(custom_name, fn_name.span())
    } else {
        fn_name.clone()
    };

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default)]
        #[doc = concat!("Auto-generated Handler from `#[risten::handler]` on `", stringify!(#fn_name), "`")]
        #fn_vis struct #struct_name;

        impl ::risten::Handler<#input_type> for #struct_name {
            type Output = #output_type;

            async fn call(&self, #input_pat: #input_type) -> Self::Output {
                #fn_block
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for async main setup with Tokio runtime.
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let expanded = quote! {
        #[::tokio::main]
        #input
    };
    TokenStream::from(expanded)
}

/// Extract handler type from variant doc comments.
/// Looks for `/// @handler(SomeHookType)` in the doc comments.
fn extract_handler_attr(attrs: &[Attribute]) -> Option<syn::Path> {
    for attr in attrs {
        if attr.path().is_ident("doc") {
            // Parse the doc string
            if let Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let content = lit_str.value();
                        // Look for @handler(TypeName)
                        if let Some(start) = content.find("@handler(") {
                            let after = &content[start + 9..];
                            if let Some(end) = after.find(')') {
                                let handler_name = after[..end].trim();
                                // Parse as a path
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

/// Derive macro to generate dispatch logic from an enum of events.
///
/// # Usage
///
/// ```rust,ignore
/// #[risten::dispatch]
/// enum AppEvent {
///     #[handler = MessageHook]  // Static hook binding
///     Message(MessageEvent),
///
///     #[handler = ReadyHook]
///     Ready(ReadyEvent),
///
///     Shutdown,  // No handler = skip
/// }
/// ```
///
/// Generates:
/// - `dispatch_match()` - Basic variant matching
/// - `variant_name()` - Get variant name as &str
/// - `dispatch_to_hooks()` - **Static** async dispatch to bound hooks
/// - `dispatch_types` module - Type aliases for inner types
///
/// # Status
///
/// This macro is **Tier 2 (Experimental)** and may change.
#[proc_macro_attribute]
pub fn dispatch(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let enum_name = &input.ident;
    let vis = &input.vis;

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => {
            return syn::Error::new_spanned(&input, "#[dispatch] can only be used on enums")
                .to_compile_error()
                .into();
        }
    };

    // Build match arms for dispatch_match (basic - always returns Next)
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

    // Build match arms for variant_name
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

    // Build STATIC dispatch arms with handler attribute
    let static_dispatch_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let handler_path = extract_handler_attr(&variant.attrs);

        match &variant.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                if let Some(handler) = handler_path {
                    // Static handler binding - call the hook directly
                    quote! {
                        #enum_name::#variant_name(inner) => {
                            let hook = #handler;
                            ::risten::Hook::on_event(&hook, inner).await
                        }
                    }
                } else {
                    // No handler - just continue
                    quote! {
                        #enum_name::#variant_name(_) => {
                            ::core::result::Result::Ok(::risten::HookResult::Next)
                        }
                    }
                }
            }
            syn::Fields::Unit => {
                // Unit variants can't have handlers (no inner data)
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

    // Generate variant type aliases
    let variant_markers = variants.iter().filter_map(|variant| {
        let variant_name = &variant.ident;
        match &variant.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let inner_type = &fields.unnamed.first().unwrap().ty;
                Some(quote! {
                    /// Type alias for this variant's inner type.
                    pub type #variant_name = #inner_type;
                })
            }
            _ => None,
        }
    });

    // Generate handler info for each variant (for inspection)
    let handler_info = variants.iter().filter_map(|variant| {
        let variant_name = &variant.ident;
        let handler_path = extract_handler_attr(&variant.attrs);
        handler_path.map(|h| {
            let handler_str = quote!(#h).to_string();
            quote! {
                /// Handler type for this variant.
                pub type #variant_name = #h;
            }
        })
    });

    let expanded = quote! {
        #input

        impl #enum_name {
            /// Basic dispatch - matches on variant, always returns Next.
            pub fn dispatch_match(&self) -> ::risten::HookResult {
                match self {
                    #(#match_arms),*
                }
            }

            /// Get the variant name as a static string.
            pub fn variant_name(&self) -> &'static str {
                match self {
                    #(#name_arms),*
                }
            }

            /// **Static** async dispatch to bound hooks.
            ///
            /// Each variant with a `/// @handler(HookType)` doc comment
            /// will have its inner data dispatched to that hook at compile time.
            /// No vtable, no dynamic dispatch - fully inlined.
            pub async fn dispatch_to_hooks(&self) -> ::core::result::Result<::risten::HookResult, ::std::boxed::Box<dyn ::std::error::Error + Send + Sync>> {
                match self {
                    #(#static_dispatch_arms),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
