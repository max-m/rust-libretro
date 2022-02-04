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

pub fn get_visibility_mut(item: &mut syn::Item) -> Option<&mut syn::Visibility> {
    match item {
        syn::Item::Const(item) => Some(&mut item.vis),
        syn::Item::Enum(item) => Some(&mut item.vis),
        syn::Item::Fn(item) => Some(&mut item.vis),
        syn::Item::Macro2(item) => Some(&mut item.vis),
        syn::Item::Mod(item) => Some(&mut item.vis),
        syn::Item::Static(item) => Some(&mut item.vis),
        syn::Item::Struct(item) => Some(&mut item.vis),
        syn::Item::Trait(item) => Some(&mut item.vis),
        syn::Item::TraitAlias(item) => Some(&mut item.vis),
        syn::Item::Type(item) => Some(&mut item.vis),
        syn::Item::Union(item) => Some(&mut item.vis),
        _ => None,
    }
}

pub fn get_attrs_mut(item: &mut syn::Item) -> Option<&mut Vec<syn::Attribute>> {
    match item {
        syn::Item::Const(item) => Some(&mut item.attrs),
        syn::Item::Enum(item) => Some(&mut item.attrs),
        syn::Item::Fn(item) => Some(&mut item.attrs),
        syn::Item::Macro(item) => Some(&mut item.attrs),
        syn::Item::Macro2(item) => Some(&mut item.attrs),
        syn::Item::Mod(item) => Some(&mut item.attrs),
        syn::Item::Static(item) => Some(&mut item.attrs),
        syn::Item::Struct(item) => Some(&mut item.attrs),
        syn::Item::Trait(item) => Some(&mut item.attrs),
        syn::Item::TraitAlias(item) => Some(&mut item.attrs),
        syn::Item::Type(item) => Some(&mut item.attrs),
        syn::Item::Union(item) => Some(&mut item.attrs),
        _ => None,
    }
}

pub fn push_attr(item: &mut syn::Item, attr: syn::Attribute) {
    if let Some(attrs) = get_attrs_mut(item) {
        attrs.push(attr);
    }
}

pub fn prepend_doc(item: &mut syn::Item, doc: &str) {
    if let Some(attrs) = get_attrs_mut(item) {
        let mut had_doc = false;

        for (index, attribute) in attrs.iter_mut().enumerate() {
            if attribute.path.is_ident("doc") {
                let doc = syn::parse_quote! {
                    #[doc = #doc]
                };

                *attrs = [&attrs[0..index], &[doc], &attrs[index..]].concat();
                had_doc = true;

                break;
            }
        }

        if !had_doc {
            push_attr(
                item,
                syn::parse_quote! {
                    #[doc = #doc]
                },
            );
        }
    }
}
