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

use crate::{
	util::{file, glue},
	Result,
};
use mashin_primitives::InternalMashinType;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Version {
	latest_version: String,
}

pub async fn write_ts(
	bindings: &PathBuf,
	out: &PathBuf,
	std_version: Option<String>,
) -> Result<()> {
	let glue: mashin_primitives::Glue = glue::get_glue(bindings)?;

	let std_version_to_use = std_version.unwrap_or(
		// grab latest STD version
		reqwest::Client::new()
			.get("https://mashin.run/api/v1/lib/std")
			.send()
			.await?
			.json::<Version>()
			.await?
			.latest_version,
	);

	let provider_doc = &glue.doc;
	let mut provider_config = None;
	let output = glue
		.type_defs
		.clone()
		.into_values()
		.map(|ty| match &ty.mashin_ty {
			InternalMashinType::ProviderConfig => {
				provider_config = Some(ty.name.clone());
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
			InternalMashinType::Resource(resource_name) => {
				let name = &ty.name;
				let output_name = format!("{}Outputs", name);
				let config_ident = format!("{}Config", name);
				let doc = &ty.doc;
				let resource_class = format!(
					r#"
{doc}export class {name}<T extends Lowercase<string>> extends MashinResource<{output_name}, T> {{
   #props: {config_ident};
   constructor(
      name: ResourceName<T>,
      props: {config_ident},
      opts: ResourceOptions
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
					"export interface {} extends Outputs {{\n{}\n}}\n{}",
					output_name, ty.typescript, resource_class
				)
			},
			InternalMashinType::Extra => {
				format!("{}export type {} = {{\n{}\n}}", ty.doc, ty.name, ty.typescript)
			},
		})
		.collect::<Vec<_>>()
		.join("\n");

	let crate_name = &glue.name.replace('-', "_");
	let crate_version = &glue.version;
	let github_url = &glue.repository;

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
import {{
	Resource as MashinResource,
	Provider as MashinProvider,
	getFileName,
	Inputs,
	Outputs,
	ResourceName,
	ResourceOptions,
 }} from "https://mashin.run/std@{std_version_to_use}/sdk/mod.ts";

export const VERSION = "{crate_version}";
const LOCAL_PATH = Deno.env.get("LOCAL_PLUGIN")
   ? "./target/debug/lib{crate_name}.dylib"
   : await globalThis.__mashin.downloadProvider(
      "github",
      new URL(
         getFileName("{crate_name}"),
         `{github_url}/releases/download/v${{VERSION}}/`
      ).toString()
   );
"#
	);

	let config_ident = provider_config.unwrap_or("Config".into());
	//let provider_ident = provider_config.unwrap_or("Provider".into()).replace("Config", "");

	let provider = format!(
		r#"
{provider_doc}export class Provider extends MashinProvider {{
   constructor(name: string, args?: {config_ident}) {{
      super(name, LOCAL_PATH as string, args);
   }}
}}
"#
	);

	let typescript = format!("{header}\n{output}\n{provider}");

	file::write_file(out, "mod.ts", typescript)
}
