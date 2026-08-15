use proc_macro::TokenStream;

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

mod pipeline;
#[proc_macro_derive(Pipeline, attributes(instance_data, ui))]
pub fn derive_pipeline(item: TokenStream) -> TokenStream {
    pipeline::derive_pipeline_impl(item)
}

mod widget;
#[proc_macro_derive(Widget, attributes(ui))]
pub fn derive_widget(item: TokenStream) -> TokenStream {
    widget::derive_widget_impl(item)
}
