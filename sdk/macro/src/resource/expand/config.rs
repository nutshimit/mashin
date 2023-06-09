/* -------------------------------------------------------- *\
 *                                                          *
 *      ███╗░░░███╗░█████╗░░██████╗██╗░░██╗██╗███╗░░██╗     *
 *      ████╗░████║██╔══██╗██╔════╝██║░░██║██║████╗░██║     *
 *      ██╔████╔██║███████║╚█████╗░███████║██║██╔██╗██║     *
 *      ██║╚██╔╝██║██╔══██║░╚═══██╗██╔══██║██║██║╚████║     *
 *      ██║░╚═╝░██║██║░░██║██████╔╝██║░░██║██║██║░╚███║     *
 *      ╚═╝░░░░░╚═╝╚═╝░░╚═╝╚═════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝     *
 *                                         by Nutshimit     *
 * -------------------------------------------------------- *
 *                                                          *
 *  This file is licensed as MIT. See LICENSE for details.  *
 *                                                          *
\* ---------------------------------------------------------*/

use crate::resource::parse::Def;

pub fn expand_config(def: &mut Def) -> proc_macro2::TokenStream {
	let resource_item = {
		let item =
			&mut def.item.content.as_mut().expect("Checked by def parser").1[def.config.index];
		if let syn::Item::Struct(item) = item {
			item
		} else {
			unreachable!("Checked by config parser")
		}
	};

	resource_item.attrs.push(syn::parse_quote! {
		#[derive(Default, Debug, Clone, ::serde::Serialize, ::serde::Deserialize, PartialEq)]
	});
	resource_item.attrs.push(syn::parse_quote! {#[serde(rename_all = "camelCase")]});

	resource_item
		.fields
		.iter_mut()
		.for_each(|field| field.attrs.push(syn::parse_quote!(#[serde(default)])));

	quote::quote! {}
}
