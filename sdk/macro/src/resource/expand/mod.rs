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

use super::parse::Def;
use crate::utils::ts::{get_glue, metafile, process_struct};
use inflector::Inflector;
use mashin_primitives::InternalMashinType;
use quote::ToTokens;
use serde_json;
use std::{env, io::Write};

mod config;
mod resource;
mod resource_impl;
mod ts;

pub fn expand(mut def: Def) -> proc_macro2::TokenStream {
	let provider_name = env::var("CARGO_PKG_NAME").unwrap_or_default();
	let mut glue = get_glue();
	let resource_name = &def.item.ident.to_string().to_pascal_case();

	// process resource before it replaced with our custom fields
	process_struct(
		&mut glue,
		&def.item.content.as_ref().expect("pre-checked").1[def.resource.index],
		InternalMashinType::Resource(def.resource.name.clone()),
		Some(format!("{resource_name}")),
	)
	.expect("valid ts");

	let config = config::expand_config(&mut def);
	let resource = resource::expand_resource(&mut def);
	let resources_impl = resource_impl::expand_resource_impl(&mut def);
	let extra_ts = ts::expand_ts(&mut def);

	// process config
	process_struct(
		&mut glue,
		&def.item.content.as_ref().expect("pre-checked").1[def.config.index],
		InternalMashinType::ResourceConfig,
		Some(format!("{resource_name}Config")),
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

	let provider_target = format!("mashin::provider::{}", provider_name);
	let new_items: proc_macro2::TokenStream = quote::quote!(
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

		#config
		#resource
		#resources_impl
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
