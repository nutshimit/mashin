use std::collections::HashMap;

use crate::provider::parse::Def;
use darling::{FromMeta, ToTokens};
use quote::quote;
use syn::{Attribute, Item, Meta};

pub fn expand_resource_config(def: &mut Def) -> proc_macro2::TokenStream {
    def.resources_config.iter_mut().for_each(|resource| {
        let resource_item = {
            let item =
                &mut def.item.content.as_mut().expect("Checked by def parser").1[resource.index];
            if let syn::Item::Struct(item) = item {
                item
            } else {
                unreachable!("Checked by config parser")
            }
        };

        resource_item.attrs.push(syn::parse_quote! {
            #[derive(Default, Debug, Clone, ::serde::Serialize, ::serde::Deserialize, PartialEq)]
        });

        resource_item
            .fields
            .iter_mut()
            .for_each(|field| field.attrs.push(syn::parse_quote!(#[serde(default)])));
    });

    quote::quote! {}
}
