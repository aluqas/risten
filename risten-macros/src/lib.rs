use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, FnArg, Ident, ItemFn, LitInt, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

// ============================================================================
// Derive Macros
// ============================================================================

/// Derive macro for implementing `Message` trait.
///
/// # Usage
///
/// ```rust,ignore
/// #[derive(Message)]
/// struct MyEvent { ... }
/// ```
///
/// This generates `impl risten::Message for MyEvent {}`.
#[proc_macro_derive(Message)]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::risten::Message for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

// ============================================================================
// Attribute Macros
// ============================================================================

/// Parsed attributes for #[risten::event(...)]
struct EventArgs {
    priority: Option<i32>,
    name: Option<String>,
    filter: Option<Expr>,
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

    // Generate filter check if filter attribute is present
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

/// Parsed attributes for #[risten::handler(...)]
struct HandlerArgs {
    name: Option<String>,
    event: Option<Type>,
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut event = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "name" => {
                    let lit: LitStr = input.parse()?;
                    name = Some(lit.value());
                }
                "event" => {
                    let ty: Type = input.parse()?;
                    event = Some(ty);
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

        Ok(HandlerArgs { name, event })
    }
}

/// Attribute macro to convert async functions into Handler implementations.
///
/// # V2 Features
///
/// - **Single argument**: Direct handler (no extraction)
/// - **Multiple arguments**: Each argument is extracted via `AsyncFromEvent`
///
/// # Usage
///
/// ## Simple Handler (single argument)
/// ```rust,ignore
/// #[handler]
/// async fn my_handler(msg: MessageEvent) -> Result<()> {
///     // msg is passed directly
/// }
/// ```
///
/// ## Extraction Handler (multiple arguments)
/// ```rust,ignore
/// #[handler(event = MessageEvent)]
/// async fn my_handler(
///     user: UserContext,    // Extracted via AsyncFromEvent<MessageEvent>
///     db: DbContext,        // Extracted via AsyncFromEvent<MessageEvent>
/// ) -> Result<()> {
///     // Both arguments are auto-extracted from the event
/// }
/// ```
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
    let arg_count = inputs.len();

    let output_type = match &input.sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => quote! { #ty },
    };

    let struct_name = if let Some(ref custom_name) = args.name {
        Ident::new(custom_name, fn_name.span())
    } else {
        fn_name.clone()
    };

    // Single argument: simple handler (no extraction)
    if arg_count == 1 && args.event.is_none() {
        let (input_pat, input_type) = match inputs.first() {
            Some(FnArg::Typed(pat_type)) => (&pat_type.pat, &pat_type.ty),
            _ => {
                return syn::Error::new_spanned(
                    inputs,
                    "Handler function must take at least one argument",
                )
                .to_compile_error()
                .into();
            }
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

        return TokenStream::from(expanded);
    }

    // Multiple arguments OR explicit event type: extraction handler
    let event_type = match args.event {
        Some(ref ty) => quote! { #ty },
        None => {
            // If no explicit event type, use the first argument's type
            match inputs.first() {
                Some(FnArg::Typed(pat_type)) => {
                    let ty = &pat_type.ty;
                    quote! { #ty }
                }
                _ => {
                    return syn::Error::new_spanned(
                        inputs,
                        "Handler must have at least one argument or specify event type",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
    };

    // Collect all arguments for extraction
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
            FnArg::Receiver(_) => {
                return syn::Error::new_spanned(arg, "Handler cannot have self parameter")
                    .to_compile_error()
                    .into();
            }
        }
    }

    // Build the function call with extracted arguments
    let arg_names: Vec<_> = (0..arg_count)
        .map(|i| Ident::new(&format!("__arg_{}", i), fn_name.span()))
        .collect();

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default)]
        #[doc = concat!("Auto-generated Handler (extraction) from `#[risten::handler]` on `", stringify!(#fn_name), "`")]
        #fn_vis struct #struct_name;

        impl ::risten::Handler<#event_type> for #struct_name {
            type Output = ::core::result::Result<#output_type, ::risten::ExtractError>;

            async fn call(&self, __event: #event_type) -> Self::Output {
                #(#extraction_code)*

                // Call the original function with extracted arguments
                async fn __inner(#(#arg_pats: #arg_types),*) -> #output_type {
                    #fn_block
                }

                ::core::result::Result::Ok(__inner(#(#arg_names),*).await)
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
    let _vis = &input.vis;

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

    // Generate variant type aliases (unused but kept for API)
    let _variant_markers = variants.iter().filter_map(|variant| {
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

    // Generate handler info for each variant (unused but kept for API)
    let _handler_info = variants.iter().filter_map(|variant| {
        let variant_name = &variant.ident;
        let handler_path = extract_handler_attr(&variant.attrs);
        handler_path.map(|h| {
            let _handler_str = quote!(#h).to_string();
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
