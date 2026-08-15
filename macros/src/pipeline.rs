use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Type, parse_macro_input, parse_quote};

use super::get_ui_path;

pub(super) fn derive_pipeline_impl(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    if !input.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &input.generics,
            "a pipeline type must be concrete: a `static` inside a generic impl \
             is one allocation shared by every instantiation, so all `Foo<T>` \
             would collide on a single registry slot",
        )
        .to_compile_error()
        .into();
    }

    let ui_path = match get_ui_path(&input.attrs) {
        Ok(path) => path,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut instance_type: Type = parse_quote!(#ui_path::primitive::Primitive);
    for attr in &input.attrs {
        if attr.path().is_ident("instance_data") {
            match attr.parse_args::<Type>() {
                Ok(ty) => instance_type = ty,
                Err(err) => return err.to_compile_error().into(),
            }
        }
    }

    quote! {
        impl #ui_path::render::pipeline::PipelineSlot for #name {
            fn slot() -> &'static ::core::sync::atomic::AtomicU32
            where
                Self: Sized,
            {
                static SLOT: ::core::sync::atomic::AtomicU32 =
                    ::core::sync::atomic::AtomicU32::new(0);
                &SLOT
            }

            fn as_any(&self) -> &dyn ::core::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }
        }
        impl #ui_path::primitive::Instanced<#instance_type> for #name {}
    }
    .into()
}
