use crate::provider::parse::Def;
use crate::provider::parse::InternalMashinType;
use crate::provider::parse::TsType;
use inflector::Inflector;
use std::collections::HashMap;
use syn::ext::IdentExt;
use syn::Attribute;
use syn::Data;
use syn::DataStruct;
use syn::DeriveInput;
use syn::Fields;
use syn::Lit;
use syn::Meta;
use syn::NestedMeta;

macro_rules! variant_instance {
    ( $variant:path, $iterator:expr ) => {
        $iterator
            .filter_map(|val| {
                if let $variant(ref f1, ref f2) = *val {
                    Some((f1, f2))
                } else {
                    None
                }
            })
            .next()
    };
}

pub fn process_struct(
    def: &mut Def,
    key: usize,
    mashin_ty: InternalMashinType,
) -> Result<(), String> {
    let input = def.item.content.clone().expect("Checked by def parser").1[key].clone();

    match &input {
        syn::Item::Struct(syn::ItemStruct {
            ident,
            attrs,
            fields: Fields::Named(fields),
            ..
        }) => {
            let fields = &fields.named;

            let name = ident;
            let mut fmap = HashMap::new();
            let mut typescript: Vec<String> = vec![];

            let serde_attrs = get_serde_attrs(attrs);

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
                    }
                    syn::Type::Reference(ref ty) => {
                        assert!(!ty.mutability.is_some());
                        assert!(ty.lifetime.is_some());
                        match *ty.elem {
                            syn::Type::Path(ref ty) => {
                                let segment = &ty.path.segments.first().unwrap();
                                let ty = segment.ident.to_string();
                                fmap.insert(ident.clone(), ty);
                            }
                            _ => unimplemented!(),
                        }
                    }
                    _ => unimplemented!(),
                };

                // force camelcase on all fields
                ident = ident.to_camel_case();

                for attr in &serde_attrs {
                    if let Some(i) = attr.transform(&ident) {
                        ident = i;
                    }
                }

                let doc_str = get_docs(&field.attrs);
                typescript.push(format!(
                    "{}  {}: {};",
                    doc_str,
                    ident,
                    types_to_ts(&field.ty)
                ));
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
        }
        syn::Item::Enum(syn::ItemEnum {
            ident,
            attrs,
            variants,
            ..
        }) => {
            let name = &ident;
            let mut typescript: Vec<String> = vec![];

            for variant in variants {
                let mut variant_fields: Vec<String> = vec![];
                let fields = &variant.fields;

                let serde_attrs = get_serde_attrs(attrs);
                for field in fields {
                    let mut ident = field
                        .ident
                        .as_ref()
                        .expect("Field without ident")
                        // Strips the raw marker `r#`, if present.
                        .unraw()
                        .to_string();

                    for attr in &serde_attrs {
                        if let Some(i) = attr.transform(&ident) {
                            ident = i;
                        }
                    }

                    let doc_str = get_docs(&field.attrs);
                    variant_fields.push(format!(
                        "{}  {}: {};",
                        doc_str,
                        ident,
                        types_to_ts(&field.ty)
                    ));
                }

                let mut ident = variant.ident.to_string();

                // Perform #[serde] attribute transformers.
                // This excludes `tag` and `content` attributes.
                // They require special treatment during codegen.
                for attr in &serde_attrs {
                    if let Some(i) = attr.transform(&ident) {
                        ident = i;
                    }
                }

                let doc_str = get_docs(&variant.attrs);

                let variant_str = if variant_fields.len() > 0 {
                    let tag_content =
                        variant_instance!(SerdeAttr::TagAndContent, serde_attrs.iter());

                    match tag_content {
                        None => {
                            format!(
                                "{} {{ {}: {{\n {}\n}} }}",
                                doc_str,
                                &ident,
                                variant_fields.join("\n")
                            )
                        }
                        Some((tag, content)) => {
                            // // $jsdoc
                            // {
                            //   $tag: $ident,
                            //   $content: { ...$fields }
                            // }
                            format!(
                                "{} {{ {}: \"{}\", {}: {{ {} }} }}",
                                doc_str,
                                &tag,
                                &ident,
                                &content,
                                variant_fields.join("\n")
                            )
                        }
                    }
                } else {
                    format!("{}  \"{}\"", doc_str, &ident)
                };

                typescript.push(variant_str);
            }

            // TODO: `type_defs` in favor of `ts_types`

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
        }
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
                &syn::PathArguments::AngleBracketed(ref args) => {
                    for p in &args.args {
                        let ty = match p {
                            syn::GenericArgument::Type(ty) => types_to_ts(ty),
                            syn::GenericArgument::Lifetime(_) => continue,
                            _ => unimplemented!(),
                        };
                        generics.push(ty);
                    }
                }
                &syn::PathArguments::None => {}
                _ => unimplemented!(),
            };

            match ty.as_ref() {
                "Option" => format!(
                    "{} | undefined | null",
                    rs_to_ts(generics.first().unwrap().as_ref())
                ),
                _ => {
                    if generics.len() > 0 {
                        let root_ty = rs_to_ts(&ty);
                        let generic_str = generics
                            .iter()
                            .map(|g| rs_to_ts(g))
                            .collect::<Vec<&str>>()
                            .join(", ");
                        format!("{}<{}>", root_ty, generic_str)
                    } else {
                        rs_to_ts(&ty).to_string()
                    }
                }
            }
        }
        _ => unimplemented!(),
    }
}

#[derive(Debug)]
pub enum SerdeAttr {
    RenameAll(String),
    TagAndContent(String, String),
}

impl SerdeAttr {
    pub fn transform(&self, s: &str) -> Option<String> {
        match self {
            SerdeAttr::RenameAll(t) => match t.as_ref() {
                "lowercase" => Some(s.to_lowercase()),
                "UPPERCASE" => Some(s.to_uppercase()),
                "camelCase" => Some(s.to_camel_case()),
                "snake_case" => Some(s.to_snake_case()),
                "PascalCase" => Some(s.to_pascal_case()),
                "SCREAMING_SNAKE_CASE" => Some(s.to_screaming_snake_case()),
                _ => panic!("Invalid attribute value: {}", s),
            },
            _ => None,
        }
    }
}

pub fn get_serde_attrs(attrs: &Vec<Attribute>) -> Vec<SerdeAttr> {
    attrs
        .iter()
        .filter(|i| i.path.is_ident("serde"))
        .flat_map(|attr| match attr.parse_meta() {
            Ok(Meta::List(l)) => l.nested.iter().find_map(|meta| match meta {
                NestedMeta::Meta(Meta::NameValue(v)) => match v.path.get_ident() {
                    Some(id) => match id.to_string().as_ref() {
                        // #[serde(rename_all = "UPPERCASE")]
                        "rename_all" => match &v.lit {
                            Lit::Str(s) => Some(SerdeAttr::RenameAll(s.value())),
                            _ => None,
                        },
                        // #[serde(tag = "key", content = "value")]
                        "tag" => match &v.lit {
                            Lit::Str(s) => {
                                let tag = s.value();

                                let lit = l.nested.iter().find_map(|meta| match meta {
                                    NestedMeta::Meta(Meta::NameValue(v)) => {
                                        match v.path.is_ident("content") {
                                            true => Some(&v.lit),
                                            false => None,
                                        }
                                    }
                                    _ => None,
                                });

                                match lit {
                                    Some(Lit::Str(s)) => {
                                        let content = s.value();
                                        Some(SerdeAttr::TagAndContent(tag, content))
                                    }
                                    _ => panic!("Missing `content` attribute with `tag`."),
                                }
                            }
                            _ => None,
                        },
                        // #[serde(content = "value", tag = "key")]
                        "content" => match &v.lit {
                            Lit::Str(s) => {
                                let content = s.value();

                                let lit = l.nested.iter().find_map(|meta| match meta {
                                    NestedMeta::Meta(Meta::NameValue(v)) => {
                                        match v.path.is_ident("tag") {
                                            true => Some(&v.lit),
                                            false => None,
                                        }
                                    }
                                    _ => None,
                                });

                                match lit {
                                    Some(Lit::Str(s)) => {
                                        let tag = s.value();
                                        Some(SerdeAttr::TagAndContent(tag, content))
                                    }
                                    _ => panic!("Missing `tag` attribute with `content`."),
                                }
                            }
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }),
            _ => None,
        })
        .collect::<Vec<_>>()
}

pub fn get_docs(attrs: &Vec<Attribute>) -> String {
    let mut doc: Vec<String> = vec![];
    for attr in attrs {
        if let Ok(Meta::NameValue(meta)) = attr.parse_meta() {
            if !meta.path.is_ident("doc") {
                continue;
            }
            if let Lit::Str(lit) = meta.lit {
                doc.push(lit.value());
            }
        }
    }

    let doc_str = if doc.len() > 0 {
        format!("/**\n  *{}\n  **/\n", doc.join("\n  *"))
    } else {
        String::new()
    };

    doc_str
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
        a @ _ => a,
    }
}
