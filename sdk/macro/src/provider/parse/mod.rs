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

use crate::utils::ts;
use syn::spanned::Spanned;

mod builder;
mod config;
mod helper;
mod provider;
mod resource;

/// Parsed definition of a provider.
pub struct Def {
	pub item: syn::ItemMod,
	pub provider: provider::ProviderDef,
	pub config: config::ConfigDef,
	pub builder: builder::BuilderDef,
	pub resources: resource::ResourceDef,
	pub extra_ts: Vec<ts::TsDef>,
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

		let mut provider = None;
		let mut config = None;

		let mut builder = None;
		let mut resources = None;
		let mut extra_ts = vec![];

		for (index, item) in items.iter_mut().enumerate() {
			let provider_attr: Option<ProviderAttr> = helper::take_first_item_provider_attr(item)?;

			// root attrs
			match provider_attr {
				Some(ProviderAttr::Provider(span)) if provider.is_none() =>
					provider = Some(provider::ProviderDef::try_from(span, index, item)?),
				Some(ProviderAttr::Config(span)) =>
					config = Some(config::ConfigDef::try_from(span, index, item)?),

				Some(ProviderAttr::Builder(span)) =>
					builder = Some(builder::BuilderDef::try_from(span, index, item)?),
				Some(ProviderAttr::Resource(span)) =>
					resources = Some(resource::ResourceDef::try_from(span, index, item)?),
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
			resources: resources
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::resource]`"))?,
			provider: provider
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::provider]`"))?,
			config: config
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::config]`"))?,
			builder: builder
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::builder]`"))?,
			extra_ts,
		};

		Ok(def)
	}
}

mod keyword {
	syn::custom_keyword!(mashin);
	syn::custom_keyword!(config);
	syn::custom_keyword!(provider);
	syn::custom_keyword!(resource);
	syn::custom_keyword!(builder);
	syn::custom_keyword!(name);
	syn::custom_keyword!(ts);
}

#[derive(Debug)]
enum ProviderAttr {
	Provider(proc_macro2::Span),
	Config(proc_macro2::Span),
	Resource(proc_macro2::Span),
	Builder(proc_macro2::Span),
	Ts(proc_macro2::Span),
}

impl ProviderAttr {
	fn span(&self) -> proc_macro2::Span {
		match self {
			Self::Provider(span) => *span,
			Self::Config(span) => *span,
			Self::Resource(span) => *span,
			Self::Builder(span) => *span,
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

		if lookahead.peek(keyword::provider) {
			return Ok(ProviderAttr::Provider(content.parse::<keyword::provider>()?.span()))
		} else if lookahead.peek(keyword::config) {
			return Ok(ProviderAttr::Config(content.parse::<keyword::config>()?.span()))
		} else if lookahead.peek(keyword::builder) {
			return Ok(ProviderAttr::Builder(content.parse::<keyword::builder>()?.span()))
		} else if lookahead.peek(keyword::ts) {
			return Ok(ProviderAttr::Ts(content.parse::<keyword::ts>()?.span()))
		} else if lookahead.peek(keyword::resource) {
			return Ok(ProviderAttr::Resource(content.parse::<keyword::resource>()?.span()))
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
