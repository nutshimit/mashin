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

use std::hash::{Hash, Hasher};
use syn::spanned::Spanned;

#[derive(Clone)]
pub struct TsDef {
	pub index: usize,
	pub attr_span: proc_macro2::Span,
}

#[derive(Clone)]
pub struct TsType {
	pub doc: String,
	pub name: String,
	pub typescript: String,
	pub mashin_ty: InternalMashinType,
	pub is_enum: bool,
}

impl PartialEq for TsType {
	fn eq(&self, other: &Self) -> bool {
		self.doc == other.doc &&
			self.name == other.name &&
			self.typescript == other.typescript &&
			self.is_enum == other.is_enum
	}
}

impl Hash for TsType {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.typescript.hash(state);
	}
}

#[derive(Clone)]
pub enum InternalMashinType {
	ProviderConfig,
	ResourceConfig,
	// should pass the resource config ident
	Resource(super::resource::ResourceDef),
	Extra,
}

impl TsDef {
	pub fn try_from(
		attr_span: proc_macro2::Span,
		index: usize,
		item: &mut syn::Item,
	) -> syn::Result<Self> {
		let _item = if let syn::Item::Struct(item) = item {
			item
		} else {
			let msg = "Invalid mashin::ts, expected struct";
			return Err(syn::Error::new(item.span(), msg))
		};

		Ok(Self { index, attr_span })
	}
}
