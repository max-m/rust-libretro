use crate::TokenStream;
use quote::quote;
use syn::Attribute;

pub fn is_public(item: &syn::Item) -> bool {
    match &item {
        syn::Item::Const(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Enum(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Fn(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Macro2(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Mod(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Static(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Struct(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Trait(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::TraitAlias(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Type(item) => matches!(item.vis, syn::Visibility::Public(_)),
        syn::Item::Union(item) => matches!(item.vis, syn::Visibility::Public(_)),
        _ => false,
    }
}

pub fn is_test(item: &syn::Item) -> bool {
    if let syn::Item::Fn(item) = item {
        for attr in &item.attrs {
            if attr.path.is_ident("test") {
                return true;
            }
        }
    }

    false
}

pub fn copy_attribute(attr: &Attribute) -> TokenStream {
    let style = if let syn::AttrStyle::Inner(token) = attr.style {
        quote! { # #token }
    } else {
        quote! { # }
    };

    let path = &attr.path;
    let tokens = &attr.tokens;

    quote! {
        #style [ #path #tokens ]
    }
}

pub fn copy_attributes(attributes: &[Attribute]) -> Vec<TokenStream> {
    let mut attrs = Vec::new();
    for attr in attributes {
        if !(attr.path.is_ident("doc") || attr.path.is_ident("deprecated")) {
            continue;
        }

        attrs.push(copy_attribute(attr));
    }

    attrs
}

pub fn prettify(source: &str) -> String {
    let file = syn::parse_file(source).unwrap();
    prettyplease::unparse(&file)
}
