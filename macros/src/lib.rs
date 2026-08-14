use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

fn get_ui_path(attrs: &[syn::Attribute]) -> syn::Result<syn::Path> {
    let mut ui_path = None;

    for attr in attrs {
        if attr.path().is_ident("ui") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate") {
                    let value = meta.value()?;
                    let string_literal: syn::LitStr = value.parse()?;
                    ui_path = Some(string_literal.parse::<syn::Path>()?);
                    Ok(())
                } else {
                    Err(meta.error("unsupported ui attribute"))
                }
            })?;
        }
    }

    Ok(match ui_path {
        Some(p) => p,
        None => syn::parse_quote!(::ui),
    })
}

#[proc_macro_derive(Pipeline, attributes(instance_data, ui))]
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

    let ui_path = match get_ui_path(&input.attrs) {
        Ok(path) => path,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut instance_type: syn::Type = syn::parse_quote!(#ui_path::primitive::Primitive);
    for attr in &input.attrs {
        if attr.path().is_ident("instance_data") {
            match attr.parse_args::<syn::Type>() {
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

#[proc_macro_derive(Widget, attributes(ui))]
pub fn derive_widget(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    let ui_path = match get_ui_path(&input.attrs) {
        Ok(path) => path,
        Err(e) => return e.to_compile_error().into(),
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics #ui_path::widget::IntoElement for #name #ty_generics #where_clause {}
    }
    .into()
}
