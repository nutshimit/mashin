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

use crate::{module_loader::TypescriptModuleLoader, util::file, Result};
use anyhow::anyhow;
use deno_core::{futures::FutureExt, serde_json};
use deno_doc::{DocNodeKind, DocParser};
use deno_graph::{
	source::{LoadFuture, LoadResponse, Loader},
	BuildOptions, CapturingModuleAnalyzer, GraphKind, ModuleGraph, ModuleSpecifier,
};
use std::{path::Path, str::FromStr};

struct SourceFileLoader {
	module_loader: TypescriptModuleLoader,
	maybe_specifier: Option<String>,
}

impl Loader for SourceFileLoader {
	fn load(&mut self, specifier: &ModuleSpecifier, _is_dynamic: bool) -> LoadFuture {
		if specifier.scheme() == "file" {
			let specifier = specifier.clone();
			let new_specifier = match &self.maybe_specifier {
				Some(new_specifier) if specifier.path().contains("mod.ts") =>
					ModuleSpecifier::from_str(&format!("{new_specifier}/mod.ts"))
						.expect("valid specifier"),
				_ => specifier.clone(),
			};

			async move {
				let file = TypescriptModuleLoader::load_from_filesystem(&specifier).await?;
				Ok(Some(LoadResponse::Module {
					specifier: new_specifier,
					maybe_headers: None,
					content: file.source,
				}))
			}
			.boxed()
		} else {
			let specifier = specifier.clone();
			let module_loader = self.module_loader.clone();
			async move {
				let file = module_loader.load_from_remote_url(&specifier, 10).await?;
				Ok(Some(LoadResponse::Module {
					specifier,
					maybe_headers: file.maybe_headers,
					content: file.source,
				}))
			}
			.boxed()
		}
	}
}

pub async fn write_docs<P, O>(
	source_file: P,
	out: O,
	module_loader: TypescriptModuleLoader,
	maybe_specifier: Option<String>,
) -> Result<()>
where
	P: AsRef<Path>,
	O: AsRef<Path>,
{
	let mut loader = SourceFileLoader { maybe_specifier, module_loader };
	let source_file =
		ModuleSpecifier::from_file_path(source_file).map_err(|_| anyhow!("invalid source file"))?;

	let analyzer = CapturingModuleAnalyzer::default();
	let mut graph = ModuleGraph::new(GraphKind::TypesOnly);
	graph
		.build(
			vec![source_file.clone()],
			&mut loader,
			BuildOptions { module_analyzer: Some(&analyzer), ..Default::default() },
		)
		.await;
	let parser = DocParser::new(graph, false, analyzer.as_capturing_parser());

	let mut doc_nodes = parser.parse_with_reexports(&source_file)?;
	doc_nodes.retain(|doc_node| doc_node.kind != DocNodeKind::Import);

	let raw_doc = serde_json::to_string(&doc_nodes)?;
	file::write_file(out, "mod.json", raw_doc)
}
