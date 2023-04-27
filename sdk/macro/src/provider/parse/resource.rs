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
 *   see LICENSE-* for license details.                     *
 *                                                          *
\* ---------------------------------------------------------*/

use syn::spanned::Spanned;

use super::get_doc_literals;

#[derive(Clone)]
pub struct ResourceDef {
	pub name: String,
	pub index: usize,
	pub attr_span: proc_macro2::Span,
	pub ident: syn::Ident,
	pub docs: Vec<syn::Expr>,
}

impl ResourceDef {
	pub fn try_from(
		attr_span: proc_macro2::Span,
		index: usize,
		item: &mut syn::Item,
	) -> syn::Result<Self> {
		let item = if let syn::Item::Enum(item) = item {
			item
		} else {
			let msg = "Invalid provider::resource, expected struct";
			return Err(syn::Error::new(item.span(), msg))
		};

		let ident = item.ident.clone();
		if !matches!(item.vis, syn::Visibility::Public(_)) {
			let msg = "Invalid provider::resource, struct must be public";
			return Err(syn::Error::new(item.span(), msg))
		}
		let docs = get_doc_literals(&item.attrs);

		Ok(Self { name: "".into(), attr_span, index, ident, docs })
	}
}
