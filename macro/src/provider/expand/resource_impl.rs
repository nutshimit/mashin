use crate::provider::parse::Def;

pub fn expand_resource_impl(def: &mut Def) -> proc_macro2::TokenStream {
	for resource in &def.resources_impl {
		let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[resource.index];
		if let syn::Item::Impl(item) = item {
			item.attrs.push(syn::parse_quote!(
				#[::mashin_sdk::ext::async_trait::async_trait]
			));
		}
	}

	quote::quote! {}
}
