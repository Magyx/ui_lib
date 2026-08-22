use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use super::get_ui_path;

pub fn derive_widget_impl(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    let ui_path = match get_ui_path(&input.attrs) {
        Ok(path) => path,
        Err(e) => return e.to_compile_error().into(),
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics #ui_path::widget::IntoElement for #name #ty_generics #where_clause {}
        impl #impl_generics #ui_path::widget::Place for #name #ty_generics #where_clause {}
    }
    .into()
}
