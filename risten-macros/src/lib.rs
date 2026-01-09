use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Type, parse_macro_input};

/// Attribute macro to convert functions into Hook implementations.
///
/// Usage:
/// ```rust,ignore
/// #[risten::event]
/// async fn my_hook(event: &MyEvent) -> Result<HookResult, BoxError> {
///     Ok(HookResult::Next)
/// }
/// ```
///
/// This generates:
/// - A unit struct `my_hook` (allow non_camel_case_types)
/// - `impl Hook<MyEvent> for my_hook` calling implementation
#[proc_macro_attribute]
pub fn event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis; // e.g. pub
    let _fn_async = &input.sig.asyncness; // must be async, verified by compiler if we use .await inside

    // Extract Event Type from the first argument
    // Expected signature: fn(event: &MyEvent)
    let inputs = &input.sig.inputs;
    let event_type = if let Some(FnArg::Typed(pat_type)) = inputs.first() {
        if let Type::Reference(type_ref) = &*pat_type.ty {
            &type_ref.elem
        } else {
            // Maybe they passed by value? Hook::on_event takes reference.
            // For now assume reference.
            &*pat_type.ty
        }
    } else {
        return syn::Error::new_spanned(inputs, "Hook function must take an event argument")
            .to_compile_error()
            .into();
    };

    // We assume the return type matches what Hook expects or can be converted if we implemented IntoResponse.
    // For now, assume it returns Result<HookResult, BoxError>.

    let struct_name = fn_name;

    // Remove #[risten::event] from the function to avoid recursion if we keep it.
    // Actually input doesn't contain the attribute being processed.

    let expanded = quote! {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy)]
        #fn_vis struct #struct_name;

        impl risten::Hook<#event_type> for #struct_name {
            async fn on_event(
                &self,
                event: &#event_type,
            ) -> Result<risten::HookResult, Box<dyn std::error::Error + Send + Sync>> {
                #fn_name(event).await
            }
        }

        #input
    };

    TokenStream::from(expanded)
}

/// Attribute macro for async main setup.
/// Wraps tokio::main.
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let expanded = quote! {
        #[tokio::main]
        #input
    };
    TokenStream::from(expanded)
}

/// Attribute macro for EnumDispatch.
///
/// Current implementation is a placeholder.
#[proc_macro_attribute]
pub fn dispatch(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    // TODO: Implement actual dispatch logic
    let expanded = quote! {
        #input
    };
    TokenStream::from(expanded)
}
