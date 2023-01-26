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

/// This macro is based on [enum-tryfrom](https://github.com/kwohlfahrt/enum-tryfrom) (MIT).
///
/// Original license:
/// > MIT License
/// >
/// > Copyright (c) 2017 Kai Wohlfahrt
/// >
/// > Permission is hereby granted, free of charge, to any person obtaining a copy
/// > of this software and associated documentation files (the "Software"), to deal
/// > in the Software without restriction, including without limitation the rights
/// > to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
/// > copies of the Software, and to permit persons to whom the Software is
/// > furnished to do so, subject to the following conditions:
/// >
/// > The above copyright notice and this permission notice shall be included in all
/// > copies or substantial portions of the Software.
/// >
/// > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
/// > IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
/// > FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
/// > AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
/// > LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
/// > OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
/// > SOFTWARE.
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

        let repr_ident = format!("{name}_REPR_TYPE");
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
