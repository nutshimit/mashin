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

use super::ProviderMetadataArgs;
use std::collections::HashMap;
use syn::spanned::Spanned;
pub use ts::{InternalMashinType, TsType};

mod builder;
mod config;
mod helper;
mod provider;
mod resource;
mod state;
mod ts;

/// Parsed definition of a provider.
#[derive(Debug)]
pub struct Def {
	pub item: syn::ItemMod,
	pub args: ProviderMetadataArgs,
	pub provider: provider::ProviderDef,
	pub config: config::ConfigDef,
	pub state: state::StateDef,
	pub builder: builder::BuilderDef,
	pub resources: Vec<resource::ResourceDef>,
	pub resources_impl: Vec<resource::ResourceImplDef>,
	pub resources_config: Vec<resource::ResourceConfigDef>,
	pub extra_ts: Vec<ts::TsDef>,
	pub type_defs: HashMap<String, TsType>,
}

impl Def {
	pub fn try_from(mut item: syn::ItemMod, args: ProviderMetadataArgs) -> syn::Result<Self> {
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
		let mut state = None;
		let mut builder = None;
		let mut resources = vec![];
		let mut resources_impl = vec![];
		let mut resources_config = vec![];
		let mut extra_ts = vec![];

		for (index, item) in items.iter_mut().enumerate() {
			let provider_attr: Option<ProviderAttr> = helper::take_first_item_provider_attr(item)?;

			// root attrs
			match provider_attr {
				Some(ProviderAttr::Provider(span)) if provider.is_none() =>
					provider = Some(provider::ProviderDef::try_from(span, index, item)?),
				Some(ProviderAttr::Config(span)) =>
					config = Some(config::ConfigDef::try_from(span, index, item)?),
				Some(ProviderAttr::State(span)) =>
					state = Some(state::StateDef::try_from(span, index, item)?),
				Some(ProviderAttr::Builder(span)) =>
					builder = Some(builder::BuilderDef::try_from(span, index, item)?),
				Some(ProviderAttr::Resource(name, config, span)) => resources
					.push(resource::ResourceDef::try_from(name, config, span, index, item)?),
				Some(ProviderAttr::ResourceImpl(span)) =>
					resources_impl.push(resource::ResourceImplDef::try_from(span, index, item)?),
				Some(ProviderAttr::ResourceConfig(span)) =>
					resources_config.push(resource::ResourceConfigDef::try_from(span, index, item)?),
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
			args,
			resources,
			resources_impl,
			resources_config,
			extra_ts,
			type_defs: Default::default(),
			provider: provider
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::resource]`"))?,
			config: config
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::config]`"))?,
			state: state.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::state]`"))?,
			builder: builder
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[mashin::builder]`"))?,
		};

		Ok(def)
	}
}

mod keyword {
	syn::custom_keyword!(mashin);
	syn::custom_keyword!(config);
	syn::custom_keyword!(state);
	syn::custom_keyword!(provider);
	syn::custom_keyword!(resource);
	syn::custom_keyword!(calls);
	syn::custom_keyword!(builder);
	syn::custom_keyword!(name);
	syn::custom_keyword!(resource_config);
	syn::custom_keyword!(ts);
}

#[derive(Debug)]
enum ProviderAttr {
	Provider(proc_macro2::Span),
	Config(proc_macro2::Span),
	State(proc_macro2::Span),
	Resource(String, syn::Ident, proc_macro2::Span),
	ResourceImpl(proc_macro2::Span),
	ResourceConfig(proc_macro2::Span),
	Builder(proc_macro2::Span),
	Ts(proc_macro2::Span),
}

impl ProviderAttr {
	fn span(&self) -> proc_macro2::Span {
		match self {
			Self::Provider(span) => *span,
			Self::Config(span) => *span,
			Self::Resource(_, _, span) => *span,
			Self::ResourceImpl(span) => *span,
			Self::ResourceConfig(span) => *span,
			Self::State(span) => *span,
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
		} else if lookahead.peek(keyword::state) {
			return Ok(ProviderAttr::State(content.parse::<keyword::state>()?.span()))
		} else if lookahead.peek(keyword::calls) {
			return Ok(ProviderAttr::ResourceImpl(content.parse::<keyword::calls>()?.span()))
		} else if lookahead.peek(keyword::resource_config) {
			return Ok(ProviderAttr::ResourceConfig(
				content.parse::<keyword::resource_config>()?.span(),
			))
		} else if lookahead.peek(keyword::ts) {
			return Ok(ProviderAttr::Ts(content.parse::<keyword::ts>()?.span()))
		} else if lookahead.peek(keyword::resource) {
			let resource = content.parse::<keyword::resource>()?.span();
			if content.peek(syn::token::Paren) {
				let generate_content;
				syn::parenthesized!(generate_content in content);
				generate_content.parse::<keyword::name>()?.span();
				generate_content.parse::<syn::Token![=]>()?.span();
				let name = generate_content.parse::<syn::LitStr>()?.value();
				generate_content.parse::<syn::Token![,]>()?.span();
				generate_content.parse::<keyword::config>()?.span();
				generate_content.parse::<syn::Token![=]>()?.span();
				let config = generate_content.parse::<syn::Ident>()?;

				return Ok(ProviderAttr::Resource(name, config, resource))
			}
		};

		Err(lookahead.error())
	}
}
