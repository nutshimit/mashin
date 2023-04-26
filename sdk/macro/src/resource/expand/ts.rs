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
 *   This file is dual-licensed as Apache-2.0 or GPL-3.0.   *
 *   see LICENSE for license details.                       *
 *                                                          *
\* ---------------------------------------------------------*/

use crate::resource::parse::Def;

pub fn expand_ts(def: &mut Def) -> proc_macro2::TokenStream {
	for item in def.extra_ts.iter() {
		let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[item.index];

		match item {
			syn::Item::Enum(item) => item.attrs.push(syn::parse_quote!(
				#[derive(Debug, serde::Deserialize, serde::Serialize)]
			)),
			syn::Item::Struct(item) => item.attrs.push(syn::parse_quote!(
				#[derive(Debug, serde::Deserialize, serde::Serialize)]
			)),
			_ => unimplemented!(),
		};
	}

	quote::quote! {}
}
