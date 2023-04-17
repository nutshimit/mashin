use crate::provider::parse::Def;

pub fn expand_builder(def: &mut Def) -> proc_macro2::TokenStream {
	let builder_item = {
		let builder = &def.builder;
		let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[builder.index];
		if let syn::Item::Impl(item) = item {
			item
		} else {
			unreachable!("Checked by config parser")
		}
	};

	builder_item.attrs.push(syn::parse_quote!(
		#[::mashin_sdk::ext::async_trait::async_trait]
	));

	quote::quote! {}
}
