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

use quote::ToTokens;
use syn::spanned::Spanned;

use super::get_doc_literals;

pub struct ConfigDef {
	pub index: usize,
	pub attr_span: proc_macro2::Span,
	pub ident: syn::Ident,
	pub docs: Vec<syn::Expr>,
}

mod keyword {
	syn::custom_keyword!(Config);
}

impl ConfigDef {
	pub fn try_from(
		attr_span: proc_macro2::Span,
		index: usize,
		item: &mut syn::Item,
	) -> syn::Result<Self> {
		let item = if let syn::Item::Struct(item) = item {
			item
		} else {
			let msg = "Invalid provider::provider, expected struct";
			return Err(syn::Error::new(item.span(), msg))
		};

		if !matches!(item.vis, syn::Visibility::Public(_)) {
			let msg = "Invalid provider::provider, struct must be public";
			return Err(syn::Error::new(item.span(), msg))
		}

		syn::parse2::<keyword::Config>(item.ident.to_token_stream())?;

		let docs = get_doc_literals(&item.attrs);

		Ok(Self { index, attr_span, ident: item.ident.clone(), docs })
	}
}
