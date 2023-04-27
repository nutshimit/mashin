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

use super::parse::Def;
use crate::utils::ts::{get_glue, metafile, process_struct};
use inflector::Inflector;
use mashin_primitives::InternalMashinType;
use quote::ToTokens;
use std::{env, io::Write};

mod builder;
mod config;
mod provider;
mod resource;
mod ts;

pub fn expand(mut def: Def) -> proc_macro2::TokenStream {
	let pkg_provider_name = env::var("CARGO_PKG_NAME").unwrap_or_default();
	let mut glue = get_glue();
	let provider_name = &def.item.ident.to_string().to_pascal_case();
	let provider = provider::expand_provider(&mut def);
	let config = config::expand_config(&mut def);
	let builder = builder::expand_builder(&mut def);
	let resources = resource::expand_resources(&mut def);
	let extra_ts = ts::expand_ts(&mut def);

	process_struct(
		&mut glue,
		&def.item.content.as_ref().expect("pre-checked").1[def.config.index],
		InternalMashinType::ProviderConfig,
		Some(format!("{provider_name}Config")),
	)
	.expect("valid ts");

	// process extra
	for ts_def in def.extra_ts.clone() {
		process_struct(
			&mut glue,
			&def.item.content.as_ref().expect("pre-checked").1[ts_def.index],
			InternalMashinType::Extra,
			None,
		)
		.expect("valid ts");
	}

	let mut metafile = metafile();
	metafile.write_all(&serde_json::to_vec(&glue).unwrap()).unwrap();

	let provider_target = format!("mashin::provider::{}", pkg_provider_name);
	let new_items = quote::quote!(
		static __MASHIN_LOG_INIT: ::std::sync::Once = std::sync::Once::new();

		#[allow(unused_macros)]
		macro_rules! log {
			($level:tt,  $patter:expr $(, $values:expr)* $(,)?) => {
				::log::$level!(
					target: #provider_target,
					$patter  $(, $values)*
				)
			};
		}
		pub(super) use log;

		#provider
		#config
		#builder
		#resources
		#extra_ts
	);

	def.item
		.content
		.as_mut()
		.expect("This is checked by parsing")
		.1
		.push(syn::Item::Verbatim(new_items));

	def.item.into_token_stream()
}
