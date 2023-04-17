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

use darling::FromMeta;
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod keyword {
	syn::custom_keyword!(dev_mode);
}
mod expand;
mod parse;

#[derive(Debug, FromMeta)]
pub struct ProviderMetadataArgs {
	github_url: String,
}

pub fn provider(
	args: proc_macro::TokenStream,
	input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
	let attr_args = parse_macro_input!(args as syn::AttributeArgs);
	let item_mod = parse_macro_input!(input as syn::ItemMod);
	let args = match ProviderMetadataArgs::from_list(&attr_args) {
		Ok(v) => v,
		Err(e) => return TokenStream::from(e.write_errors()),
	};

	match parse::Def::try_from(item_mod, args) {
		Ok(def) => expand::expand(def).into(),
		Err(e) => e.to_compile_error().into(),
	}
}
