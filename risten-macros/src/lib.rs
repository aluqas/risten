use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, FnArg, Ident, ItemFn, LitInt, LitStr, Meta, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Derive macro for implementing `Message` trait.
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

// ... existing code ...

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

// ... EventArgs, HandlerArgs ...

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

fn generate_handler_impl(
    input: &ItemFn,
    args: &HandlerArgs,
) -> (proc_macro2::TokenStream, Type) {
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let is_async = input.sig.asyncness.is_some();
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

    if arg_count == 1 && args.event.is_none() {
        let (input_pat, input_type) = match inputs.first().unwrap() {
            FnArg::Typed(pat_type) => (&pat_type.pat, &pat_type.ty),
            _ => panic!("Handler function must take at least one argument"),
        };

        let call_body = if is_async {
            quote! { #fn_block }
        } else {
            quote! { { #fn_block } }
        };

        let impl_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy, Debug, Default)]
            #[doc = concat!("Auto-generated Handler from `#[risten::handler]` on `", stringify!(#fn_name), "`")]
            #fn_vis struct #struct_name;

            impl ::risten::Handler<#input_type> for #struct_name {
                type Output = #output_type;

                async fn call(&self, #input_pat: #input_type) -> Self::Output {
                    #call_body
                }
            }
        };

        let event_type = *input_type.clone();
        return (impl_code, event_type);
    }

    let event_type = match args.event {
        Some(ref ty) => quote! { #ty },
        None => {
            match inputs.first().unwrap() {
                FnArg::Typed(pat_type) => {
                    let ty = &pat_type.ty;
                    quote! { #ty }
                }
                _ => panic!("Handler must have at least one argument or specify event type"),
            }
        }
    };

    let parsed_event_type: Type = syn::parse2(event_type.clone()).unwrap();

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

                if is_async {
                    extraction_code.push(quote! {
                        let #arg_name: #ty = <#ty as ::risten::AsyncFromEvent<_>>::from_event(&__event)
                            .await
                            .map_err(|e| ::risten::ExtractError::new(e.to_string()))?;
                    });
                } else {
                    extraction_code.push(quote! {
                        let #arg_name: #ty = <#ty as ::risten::FromEvent<_>>::from_event(&__event)
                            .map_err(|e| ::risten::ExtractError::new(e.to_string()))?;
                    });
                }
            }
            FnArg::Receiver(_) => panic!("Handler cannot have self parameter"),
        }
    }

    let arg_names: Vec<_> = (0..arg_count)
        .map(|i| Ident::new(&format!("__arg_{}", i), fn_name.span()))
        .collect();

    let inner_call = if is_async {
        quote! {
            async fn __inner(#(#arg_pats: #arg_types),*) -> #output_type {
                #fn_block
            }
            ::core::result::Result::Ok(__inner(#(#arg_names),*).await)
        }
    } else {
        quote! {
            fn __inner(#(#arg_pats: #arg_types),*) -> #output_type {
                #fn_block
            }
            ::core::result::Result::Ok(__inner(#(#arg_names),*))
        }
    };

    let impl_code = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default)]
        #[doc = concat!("Auto-generated Handler (extraction) from `#[risten::handler]` on `", stringify!(#fn_name), "`")]
        #fn_vis struct #struct_name;

        impl ::risten::Handler<#event_type> for #struct_name {
            type Output = ::core::result::Result<#output_type, ::risten::ExtractError>;

            async fn call(&self, __event: #event_type) -> Self::Output {
                #(#extraction_code)*
                #inner_call
            }
        }
    };

    (impl_code, parsed_event_type)
}

#[proc_macro_attribute]
pub fn handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as HandlerArgs);
    let input = parse_macro_input!(item as ItemFn);

    let (expanded, _) = generate_handler_impl(&input, &args);
    TokenStream::from(expanded)
}

struct SubscribeArgs {
    event_type: Option<Type>,
}

impl Parse for SubscribeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let event_type = if input.is_empty() {
            None
        } else {
            Some(input.parse()?)
        };
        Ok(SubscribeArgs { event_type })
    }
}

#[proc_macro_attribute]
pub fn subscribe(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as SubscribeArgs);
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;

    let handler_args = HandlerArgs {
        name: None,
        event: args.event_type.clone(),
    };

    let (handler_impl, event_type) = generate_handler_impl(&input, &handler_args);
    let handler_struct_name = fn_name;

    let static_name = Ident::new(&format!("__HANDLER_INSTANCE_{}", fn_name).to_uppercase(), fn_name.span());
    let wrapper_name = Ident::new(&format!("__HANDLER_WRAPPER_{}", fn_name).to_uppercase(), fn_name.span());

    // Generate code using ErasedHandlerWrapper
    // We assume risten::routing::ErasedHandlerWrapper is available
    let submit_code = quote! {
        // Instantiate the handler
        static #static_name: #handler_struct_name = #handler_struct_name;

        // Wrap it in ErasedHandlerWrapper
        static #wrapper_name: ::risten::routing::ErasedHandlerWrapper<#event_type, #handler_struct_name> =
            ::risten::routing::ErasedHandlerWrapper::new(#handler_struct_name);

        ::risten::inventory::submit! {
            ::risten::routing::HandlerRegistration {
                type_id: ::std::any::TypeId::of::<#event_type>(),
                handler: &#wrapper_name,
                priority: 0,
            }
        }
    };

    let expanded = quote! {
        #handler_impl
        #submit_code
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn on(attr: TokenStream, item: TokenStream) -> TokenStream {
    subscribe(attr, item)
}

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let expanded = quote! {
        #[::tokio::main]
        #input
    };
    TokenStream::from(expanded)
}

fn extract_handler_attr(attrs: &[Attribute]) -> Option<syn::Path> {
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
            pub fn dispatch_match(&self) -> ::risten::HookResult {
                match self {
                    #(#match_arms),*
                }
            }

            pub fn variant_name(&self) -> &'static str {
                match self {
                    #(#name_arms),*
                }
            }

            pub async fn dispatch_to_hooks(&self) -> ::core::result::Result<::risten::HookResult, ::std::boxed::Box<dyn ::std::error::Error + Send + Sync>> {
                match self {
                    #(#static_dispatch_arms),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
