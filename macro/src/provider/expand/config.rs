use super::helper::process_struct;
use crate::provider::parse::Def;
use darling::ToTokens;
use quote::quote;
use syn::{Attribute, Meta};

pub fn expand_config(def: &mut Def) -> proc_macro2::TokenStream {
    let config_item = {
        let config = &def.config;
        let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[config.index];
        if let syn::Item::Struct(item) = item {
            item
        } else {
            unreachable!("Checked by config parser")
        }
    };

    let ident = &config_item.ident;

    config_item.attrs.push(
        syn::parse_quote!(#[derive(Debug, Default, ::serde::Serialize, ::serde::Deserialize)]),
    );

    config_item
        .attrs
        .push(syn::parse_quote!(#[serde(rename_all = "camelCase")]));

    quote::quote! {
        impl ::mashin_sdk::Config for #ident {}
    }
}
