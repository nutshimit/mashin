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

use crate::provider::parse::Def;

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

	// let ident = &config_item.ident;

	config_item.attrs.push(
		syn::parse_quote!(#[derive(Debug, Default, ::serde::Serialize, ::serde::Deserialize)]),
	);

	config_item.attrs.push(syn::parse_quote!(#[serde(rename_all = "camelCase")]));

	quote::quote! {}
}
