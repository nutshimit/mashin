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

    config_item.attrs.push(syn::parse_quote!(
        #[derive(
            Default
        )]
    ));

    quote::quote!()
}
