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

use anyhow::Result;
use mashin_primitives::{Glue, InternalMashinType};
use serde::Deserialize;

#[derive(Deserialize)]
struct Version {
	latest: String,
	#[allow(dead_code)]
	availables: Vec<String>,
}

pub async fn generate_ts(glue: &Glue) -> Result<String> {
	// grab latest SDK version
	let latest_sdk = reqwest::Client::new()
		.get("https://mashin.run/std.json")
		.send()
		.await?
		.json::<Version>()
		.await?
		.latest;

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
					"{}export interface {} extends Outputs {{\n{}\n}}\n{}",
					ty.doc, output_name, ty.typescript, resource_class
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
import * as resource from "https://mashin.run/std@{latest_sdk}/sdk/resource.ts";
import {{ Inputs, Outputs }} from "https://mashin.run/std@{latest_sdk}/sdk/output.ts";
import {{ getFileName }} from "https://mashin.run/std@{latest_sdk}/sdk/download.ts";

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
export class Provider extends resource.Provider {{
    constructor(name: string, args?: {config_ident}) {{
      super(name, LOCAL_PATH, args);
    }}
}}
"#
	);

	Ok(format!("{header}\n{output}\n{provider}"))
}
