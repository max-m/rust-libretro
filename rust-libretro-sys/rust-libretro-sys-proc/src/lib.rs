#![doc(
    html_logo_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/max-m/rust-libretro/master/media/favicon.png"
)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{DataEnum, DeriveInput, Meta, Meta::List, MetaList, NestedMeta};

fn filter_primitive_type_attr(attr: &syn::Attribute) -> Option<(String, Span)> {
    if let Ok(List(MetaList {
        path,
        paren_token: _,
        nested,
    })) = attr.parse_meta()
    {
        if let Some(ident) = path.get_ident() {
            if ident == "repr" {
                if let Some(NestedMeta::Meta(Meta::Path(path))) = nested.first() {
                    if let Some(ident) = path.get_ident() {
                        return Some((ident.to_string(), ident.span()));
                    }
                }
            }
        }
    }

    None
}

#[proc_macro_derive(TryFromPrimitive)]
pub fn from_primitive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    let name = &input.ident;

    let types = input
        .attrs
        .iter()
        .filter_map(filter_primitive_type_attr)
        .map(|(ident, span)| syn::Ident::new(&ident, span));

    let variants = if let syn::Data::Enum(DataEnum { ref variants, .. }) = input.data {
        variants
    } else {
        panic!("`TryFromPrimitive` is only supported on Enums")
    };

    let impls = types.map(|ty| {
        let blocks = variants.iter().map(|var| {
            let ident = &var.ident;
            if !matches!(var.fields, syn::Fields::Unit) {
                panic!("Enum variant may not store data!")
            }

            quote! {
                x if x == #name::#ident as #ty => Ok(#name::#ident)
            }
        });

        let repr_ident = format!("{}_REPR_TYPE", name);
        let repr_ident = syn::Ident::new(&repr_ident, name.span());

        let tokens = quote! {
            pub type #repr_ident = #ty;

            impl TryFrom<#ty> for #name {
                type Error = crate::InvalidEnumValue<#ty>;

                fn try_from(v: #ty) -> Result<Self, Self::Error> {
                    match v {
                        #(#blocks,)*
                        v => Err(Self::Error::new(v))
                    }
                }
            }
        };

        tokens
    });

    quote! {
        #(#impls)*
    }
    .into()
}
