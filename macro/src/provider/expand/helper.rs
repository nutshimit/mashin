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

use crate::provider::parse::{Def, InternalMashinType, TsType};
use inflector::Inflector;
use std::collections::HashMap;
use syn::{ext::IdentExt, Attribute, Fields, Meta};

pub fn process_struct(
	def: &mut Def,
	key: usize,
	mashin_ty: InternalMashinType,
) -> Result<(), String> {
	let input = def.item.content.clone().expect("Checked by def parser").1[key].clone();

	match &input {
		syn::Item::Struct(syn::ItemStruct {
			ident, attrs, fields: Fields::Named(fields), ..
		}) => {
			let fields = &fields.named;

			let name = ident;
			let mut fmap = HashMap::new();
			let mut typescript: Vec<String> = vec![];

			//let serde_attrs = get_serde_attrs(attrs);

			for field in fields.iter() {
				let mut ident = field
					.ident
					.as_ref()
					.expect("Field without ident")
					// Strips the raw marker `r#`, if present.
					.unraw()
					.to_string();

				match field.ty {
					syn::Type::Path(ref ty) => {
						let segment = &ty.path.segments.first().unwrap();
						let ty = segment.ident.to_string();
						fmap.insert(ident.clone(), ty);
					},
					syn::Type::Reference(ref ty) => {
						assert!(ty.mutability.is_none());
						assert!(ty.lifetime.is_some());
						match *ty.elem {
							syn::Type::Path(ref ty) => {
								let segment = &ty.path.segments.first().unwrap();
								let ty = segment.ident.to_string();
								fmap.insert(ident.clone(), ty);
							},
							_ => unimplemented!(),
						}
					},
					_ => unimplemented!(),
				};

				// force camelcase on all fields
				ident = ident.to_camel_case();

				/*
				for attr in &serde_attrs {
					if let Some(i) = attr.transform(&ident) {
						ident = i;
					}
				}
				*/

				let doc_str = get_docs(&field.attrs);
				typescript.push(format!("{}  {}: {};", doc_str, ident, types_to_ts(&field.ty)));
			}

			let doc_str = get_docs(attrs);

			def.type_defs.insert(
				name.to_string(),
				TsType {
					doc: doc_str,
					name: name.to_string(),
					typescript: typescript.join("\n"),
					mashin_ty,
					is_enum: false,
				},
			);

			Ok(())
		},
		syn::Item::Enum(syn::ItemEnum { ident, attrs, variants, .. }) => {
			let name = &ident;
			let mut typescript: Vec<String> = vec![];

			for variant in variants {
				let mut variant_fields: Vec<String> = vec![];
				let fields = &variant.fields;

				//let serde_attrs = get_serde_attrs(attrs);
				for field in fields {
					let ident = field
						.ident
						.as_ref()
						.expect("Field without ident")
						// Strips the raw marker `r#`, if present.
						.unraw()
						.to_string();

					/*
					for attr in &serde_attrs {
						if let Some(i) = attr.transform(&ident) {
							ident = i;
						}
					}
					*/

					let doc_str = get_docs(&field.attrs);
					variant_fields.push(format!(
						"{}  {}: {};",
						doc_str,
						ident,
						types_to_ts(&field.ty)
					));
				}

				let ident = variant.ident.to_string();

				// Perform #[serde] attribute transformers.
				// This excludes `tag` and `content` attributes.
				// They require special treatment during codegen.
				/*
				for attr in &serde_attrs {
					if let Some(i) = attr.transform(&ident) {
						ident = i;
					}
				}
				*/

				let doc_str = get_docs(&variant.attrs);

				let variant_str = if !variant_fields.is_empty() {
					format!("{} {{ {}: {{\n {}\n}} }}", doc_str, &ident, variant_fields.join("\n"))
				} else {
					format!("{}  \"{}\"", doc_str, &ident)
				};

				typescript.push(variant_str);
			}

			let doc_str = get_docs(attrs);

			def.type_defs.insert(
				name.to_string(),
				TsType {
					doc: doc_str,
					name: name.to_string(),
					typescript: typescript.join("  |\n"),
					mashin_ty,
					is_enum: true,
				},
			);
			Ok(())
		},
		_ => unimplemented!(),
	}
}

fn types_to_ts(ty: &syn::Type) -> String {
	match ty {
		syn::Type::Array(_) => String::from("any"),
		syn::Type::Ptr(_) => String::from("any"),
		syn::Type::Reference(ref ty) => types_to_ts(&ty.elem),
		syn::Type::Path(ref ty) => {
			// std::alloc::Vec => Vec
			let segment = &ty.path.segments.last().unwrap();
			let ty = segment.ident.to_string();
			let mut generics: Vec<String> = vec![];
			let generic_params = &segment.arguments;
			match generic_params {
				syn::PathArguments::AngleBracketed(args) =>
					for p in &args.args {
						let ty = match p {
							syn::GenericArgument::Type(ty) => types_to_ts(ty),
							syn::GenericArgument::Lifetime(_) => continue,
							_ => unimplemented!(),
						};
						generics.push(ty);
					},
				&syn::PathArguments::None => {},
				_ => unimplemented!(),
			};

			match ty.as_ref() {
				"Option" =>
					format!("{} | undefined | null", rs_to_ts(generics.first().unwrap().as_ref())),
				_ =>
					if !generics.is_empty() {
						let root_ty = rs_to_ts(&ty);
						let generic_str =
							generics.iter().map(|g| rs_to_ts(g)).collect::<Vec<&str>>().join(", ");
						format!("{}<{}>", root_ty, generic_str)
					} else {
						rs_to_ts(&ty).to_string()
					},
			}
		},
		_ => unimplemented!(),
	}
}

pub fn get_docs(attrs: &Vec<Attribute>) -> String {
	let mut doc: Vec<String> = vec![];
	for attr in attrs {
		if let Meta::NameValue(meta) = &attr.meta {
			if !meta.path.is_ident("doc") {
				continue
			}
			if let syn::Expr::Lit(lit) = &meta.value {
				if let syn::Lit::Str(raw_doc) = &lit.lit {
					doc.push(raw_doc.value());
				}
			}
		}
	}

	if !doc.is_empty() {
		format!("/**\n  *{}\n  **/\n", doc.join("\n  *"))
	} else {
		String::new()
	}
}

fn rs_to_ts(ty: &str) -> &str {
	match ty {
		"i8" => "number",
		"i16" => "number",
		"i32" => "number",
		"i64" => "number",
		"u8" => "number",
		"u16" => "number",
		"u32" => "number",
		"u64" => "number",
		"usize" => "number",
		"bool" => "boolean",
		"String" => "string",
		"str" => "string",
		"f32" => "number",
		"f64" => "number",
		"HashMap" => "Record",
		"Vec" => "Array",
		"HashSet" => "Array",
		"Value" => "any",
		a => a,
	}
}
