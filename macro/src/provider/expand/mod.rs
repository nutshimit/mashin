use super::parse::Def;
use quote::ToTokens;

mod builder;
mod config;
mod provider;
mod resource;
mod resource_config;
mod resource_impl;

pub fn expand(mut def: Def) -> proc_macro2::TokenStream {
    let provider = provider::expand_provider(&mut def);
    let config = config::expand_config(&mut def);
    let builder = builder::expand_builder(&mut def);
    let resources = resource::expand_resources(&mut def);
    let resources_impl = resource_impl::expand_resource_impl(&mut def);
    let resources_config = resource_config::expand_resource_config(&mut def);

    let new_items = quote::quote!(
        use ::serde::ser::SerializeStruct as _;
        static __MASHIN_LOG_INIT: ::std::sync::Once = std::sync::Once::new();

        #provider
        #config
        #builder
        #resources
        #resources_impl
        #resources_config


    );

    def.item
        .content
        .as_mut()
        .expect("This is checked by parsing")
        .1
        .push(syn::Item::Verbatim(new_items));

    def.item.into_token_stream()
}
