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

use quote::ToTokens;
use syn::spanned::Spanned;

use super::get_doc_literals;

#[derive(Clone)]
pub struct ResourceDef {
	pub name: String,
	pub index: usize,
	pub attr_span: proc_macro2::Span,
	pub docs: Vec<syn::Expr>,
}

mod keyword {
	syn::custom_keyword!(Resource);
}

impl ResourceDef {
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

		let docs = get_doc_literals(&item.attrs);

		syn::parse2::<keyword::Resource>(item.ident.to_token_stream())?;

		Ok(Self { name: "".into(), index, attr_span, docs })
	}
}

/// Input definition for the pallet builder.
#[derive(Debug)]
pub struct ResourceImplDef {
	pub index: usize,
	/// The span of the pallet::builder attribute.
	pub attr_span: proc_macro2::Span,
}

impl ResourceImplDef {
	pub fn try_from(
		attr_span: proc_macro2::Span,
		index: usize,
		item: &mut syn::Item,
	) -> syn::Result<Self> {
		let _item = if let syn::Item::Impl(item) = item {
			item
		} else {
			let msg = "Invalid mashin::builder, expected struct";
			return Err(syn::Error::new(item.span(), msg))
		};

		Ok(Self { index, attr_span })
	}
}
