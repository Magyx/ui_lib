use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Pipeline, attributes(instance_data))]
pub fn derive_pipeline(item: TokenStream) -> TokenStream {
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

    let mut instance_type: syn::Type = syn::parse_quote!(::ui::primitive::Primitive);
    for attr in &input.attrs {
        if attr.path().is_ident("instance_data") {
            // Note: If using syn 1.x, use `attr.path.is_ident`
            match attr.parse_args::<syn::Type>() {
                Ok(ty) => instance_type = ty,
                Err(err) => return err.to_compile_error().into(),
            }
        }
    }

    quote! {
        impl ::ui::render::pipeline::PipelineSlot for #name {
            fn slot() -> &'static ::core::sync::atomic::AtomicU32
            where
                Self: Sized,
            {
                static SLOT: ::core::sync::atomic::AtomicU32 =
                    ::core::sync::atomic::AtomicU32::new(0);
                &SLOT
            }

            fn as_any_mut(&mut self) -> &mut dyn ::core::any::Any {
                self
            }
        }
        impl ::ui::primitive::Instanced<#instance_type> for #name {}
    }
    .into()
}
