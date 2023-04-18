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

use super::helper::process_struct;
use crate::provider::parse::{Def, InternalMashinType};
use inflector::Inflector;
use std::{
	collections::hash_map::DefaultHasher,
	env,
	fs::OpenOptions,
	hash::{Hash, Hasher},
	io::{Read, Write},
	path::Path,
};

pub fn expand_ts(def: &mut Def) -> proc_macro2::TokenStream {
	for item in def.extra_ts.iter() {
		let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[item.index];

		match item {
			syn::Item::Enum(item) => item.attrs.push(syn::parse_quote!(
				#[derive(Debug, serde::Deserialize, serde::Serialize)]
			)),
			syn::Item::Struct(item) => item.attrs.push(syn::parse_quote!(
				#[derive(Debug, serde::Deserialize, serde::Serialize)]
			)),
			_ => unimplemented!(),
		};
	}

	quote::quote! {}
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
	let mut s = DefaultHasher::new();
	t.hash(&mut s);
	s.finish()
}

fn get_cached_hash(hash_path: &str) -> String {
	match OpenOptions::new().read(true).open(hash_path) {
		Ok(mut fd) => {
			let mut meta = String::new();
			fd.read_to_string(&mut meta).expect("Error reading meta file");

			meta
		},
		Err(_) => "LOCK_NOT_FOUND".to_string(),
	}
}

fn write_to_file(out_dir: &str, final_output: &str) {
	let mut metafile = OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.open(out_dir)
		.expect("Error opening meta file");

	metafile.write_all(final_output.as_bytes()).unwrap();
}

pub fn export_ts(def: &mut Def) {
	let hash_path: String = match env::var("TARGET") {
		Ok(out_dir) => Path::new(&out_dir).join("mod.lock").into_os_string().into_string().unwrap(),
		Err(_e) => String::from("mod.lock"),
	};

	let out_dir: String = match env::var("TARGET") {
		Ok(out_dir) => Path::new(&out_dir).join("mod.ts").into_os_string().into_string().unwrap(),
		Err(_e) => String::from("mod.ts"),
	};

	// check if we need to rebuild
	let mut hash = "".to_string();
	for (_, val) in def.type_defs.iter() {
		hash = format!("{hash}{}", calculate_hash(val));
	}

	if get_cached_hash(&hash_path) == hash && Path::new(&out_dir).exists() {
		return
	}

	// process config
	process_struct(def, def.config.index, InternalMashinType::ProviderConfig).expect("valid ts");

	// process extra
	for ts_def in def.extra_ts.clone() {
		process_struct(def, ts_def.index, InternalMashinType::Extra).expect("valid ts");
	}

	// process resource configs
	for config in def.resources_config.clone().iter() {
		process_struct(def, config.index, InternalMashinType::ResourceConfig).expect("valid ts");
	}

	let output = def
		.type_defs
		.clone()
		.into_values()
		.map(|ty| match ty.mashin_ty {
			InternalMashinType::ProviderConfig => {
				format!(
					"{}export interface {} extends Inputs {{\n{}\n}}",
					ty.doc, ty.name, ty.typescript
				)
			},
			InternalMashinType::ResourceConfig => {
				format!(
					"{}export interface {} extends Inputs {{\n{}\n}}",
					ty.doc, ty.name, ty.typescript
				)
			},
			InternalMashinType::Resource(resource) => {
				let name = ty.name;
				let output_name = format!("{}Outputs", name);
				let config_ident = resource.config.to_string().to_pascal_case();
				let resource_name = resource.name;

				let resource_class = format!(
					r#"
export class {name}<T extends Lowercase<string>> extends resource.Resource<
{output_name},
T
> {{
    #props: {config_ident};
    constructor(
        name: resource.ResourceName<T>,
        props: {config_ident},
        opts: resource.ResourceOptions
    ) {{
        super(name, "{resource_name}", props, opts);
        this.#props = props;
    }}

    get props() {{
        return this.#props;
    }}
}}
"#
				);
				format!(
					"{}export interface {} extends Outputs {{\n{}\n}};\n{}",
					ty.doc, output_name, ty.typescript, resource_class
				)
			},
			InternalMashinType::Extra => {
				format!("{}export type {} = {{\n{}\n}};", ty.doc, ty.name, ty.typescript)
			},
		})
		.collect::<Vec<_>>()
		.join("\n");

	let crate_name = match env::var("MASHIN_PKG_NAME") {
		Ok(version) => version,
		Err(_e) => env!("CARGO_PKG_NAME").to_string(),
	};

	let _crate_version = match env::var("MASHIN_PKG_VERSION") {
		Ok(version) => version,
		Err(_e) => env!("CARGO_PKG_VERSION").to_string(),
	};

	let _github_url = &def.args.github_url;

	let header = format!(
		r#"/* -------------------------------------------------------- *\
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
*   This file is generated automatically by mashin.        *
*   Do not edit manually.                                  *
*                                                          *
\* ---------------------------------------------------------*/

import * as resource from "https://mashin.land/sdk/resource.ts";
import {{ Inputs, Outputs }} from "https://mashin.land/sdk/output.ts";

const url = Deno.env.get("LOCAL_PLUGIN")
  ? "./target/debug/lib{}.dylib"
  : await globalThis.__mashin.downloadProvider(
      "github", "https://github.com/lemarier/tauri-test/releases/download/v2.0.0/libatmosphere_test.dylib"
    );
"#,
		crate_name.replace('-', "_")
	);

	let provider = r#"
export class Provider extends resource.Provider {
    constructor(name: string, args?: Config) {
      // FIXME: have dynamic provider path for each OS
      super(name, url, args);
    }
}
"#
	.to_string();

	let final_output = format!("{header}\n{output}\n{provider}\n// {hash}");

	write_to_file(&out_dir, &final_output);
	write_to_file(&hash_path, &hash);
}
