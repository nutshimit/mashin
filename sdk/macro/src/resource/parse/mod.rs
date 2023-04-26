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

use crate::utils::ts;
use syn::spanned::Spanned;

mod config;
mod helper;
mod resource;

/// Parsed definition of a provider.
pub struct Def {
	pub(crate) item: syn::ItemMod,
	pub(crate) config: config::ConfigDef,
	pub(crate) resource: resource::ResourceDef,
	pub(crate) resource_calls: resource::ResourceImplDef,
	pub(crate) extra_ts: Vec<ts::TsDef>,
}

impl Def {
	pub fn try_from(mut item: syn::ItemMod) -> syn::Result<Self> {
		let item_span = item.span();
		let items = &mut item
			.content
			.as_mut()
			.ok_or_else(|| {
				let msg = "Invalid pallet definition, expected mod to be inlined.";
				syn::Error::new(item_span, msg)
			})?
			.1;

		let mut config = None;
		let mut resource = None;
		let mut resource_calls = None;
		let mut extra_ts = vec![];

		for (index, item) in items.iter_mut().enumerate() {
			let resource_attr: Option<ProviderAttr> = helper::take_first_item_provider_attr(item)?;

			// root attrs
			match resource_attr {
				Some(ProviderAttr::Resource(span)) if resource.is_none() =>
					resource = Some(resource::ResourceDef::try_from(span, index, item)?),
				Some(ProviderAttr::Config(span)) if config.is_none() =>
					config = Some(config::ConfigDef::try_from(span, index, item)?),
				Some(ProviderAttr::ResourceImpl(span)) if resource_calls.is_none() =>
					resource_calls = Some(resource::ResourceImplDef::try_from(span, index, item)?),
				Some(ProviderAttr::Ts(span)) =>
					extra_ts.push(ts::TsDef::try_from(span, index, item)?),
				Some(attr) => {
					let msg = "Invalid duplicated attribute";
					return Err(syn::Error::new(attr.span(), msg))
				},
				None => (),
			}
		}

		let def = Def {
			item,
			resource: resource
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::resource]`"))?,
			resource_calls: resource_calls
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::calls]`"))?,

			extra_ts,
			config: config
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::config]`"))?,
		};

		Ok(def)
	}
}

mod keyword {
	syn::custom_keyword!(mashin);
	syn::custom_keyword!(config);
	syn::custom_keyword!(resource);
	syn::custom_keyword!(calls);
	syn::custom_keyword!(ts);
}

#[derive(Debug)]
enum ProviderAttr {
	Resource(proc_macro2::Span),
	ResourceImpl(proc_macro2::Span),
	Config(proc_macro2::Span),
	Ts(proc_macro2::Span),
}

impl ProviderAttr {
	fn span(&self) -> proc_macro2::Span {
		match self {
			Self::Resource(span) => *span,
			Self::Config(span) => *span,
			Self::ResourceImpl(span) => *span,
			Self::Ts(span) => *span,
		}
	}
}

impl syn::parse::Parse for ProviderAttr {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		input.parse::<syn::Token![#]>()?;
		let content;
		syn::bracketed!(content in input);
		content.parse::<keyword::mashin>()?;
		content.parse::<syn::Token![::]>()?;

		let lookahead = content.lookahead1();

		if lookahead.peek(keyword::resource) {
			return Ok(ProviderAttr::Resource(content.parse::<keyword::resource>()?.span()))
		} else if lookahead.peek(keyword::config) {
			return Ok(ProviderAttr::Config(content.parse::<keyword::config>()?.span()))
		} else if lookahead.peek(keyword::calls) {
			return Ok(ProviderAttr::ResourceImpl(content.parse::<keyword::calls>()?.span()))
		} else if lookahead.peek(keyword::ts) {
			return Ok(ProviderAttr::Ts(content.parse::<keyword::ts>()?.span()))
		};

		Err(lookahead.error())
	}
}

/// Return all doc attributes literals found.
pub fn get_doc_literals(attrs: &[syn::Attribute]) -> Vec<syn::Expr> {
	attrs
		.iter()
		.filter_map(|attr| {
			if let syn::Meta::NameValue(meta) = &attr.meta {
				meta.path
					.get_ident()
					.filter(|ident| *ident == "doc")
					.map(|_| meta.value.clone())
			} else {
				None
			}
		})
		.collect()
}
