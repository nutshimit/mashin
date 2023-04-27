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

use crate::resource::parse::Def;
use inflector::Inflector;
use quote::{format_ident, quote};
use syn::{Item, Meta};

pub fn expand_resource(def: &mut Def) -> proc_macro2::TokenStream {
	let mod_ident = &def.item.ident;
	let mut resource_item = {
		let item =
			&mut def.item.content.as_mut().expect("Checked by def parser").1[def.resource.index];
		let item_cloned = item.clone();
		*item = Item::Verbatim(quote::quote!());
		if let syn::Item::Struct(item) = item_cloned {
			item
		} else {
			unreachable!("Checked by config parser")
		}
	};
	let resource = &def.resource;

	let resource_ident = &resource_item.ident;
	let isolated_ident = format_ident!("{}", &resource_ident.to_string().to_snake_case());

	let resource_name_str = resource_item.ident.to_string();

	let mut fields_json = Vec::new();
	let mut fields_helpers_impl = Vec::new();

	for field in resource_item.fields.iter_mut() {
		let name = &field.ident.clone().expect("valid name");
		let field_ty = &field.ty;

		let mut sensitive = false;
		let mut attr_id = 0;

		for attribute in field.attrs.clone() {
			if let Meta::Path(path) = attribute.meta {
				if path.is_ident("sensitive") {
					sensitive = true;
					field.attrs.remove(attr_id);
				}
				attr_id += 1;
			}
		}

		let field_name = name.to_string();
		let field_setter_fn = format_ident!("set_{}", &field_name);
		let field_getter_fn = format_ident!("{}", &field_name);
		let field_json = quote! {
			  &::mashin_sdk::ext::serde_json::json! {
				   {
						"__value": self.#name,
						"__sensitive": #sensitive,
				   }
			  }
		};

		fields_json.push(quote! { state.serialize_field(#field_name, #field_json)?; });

		fields_helpers_impl.push(quote! {
			  pub fn #field_setter_fn(&mut self, value: #field_ty) -> &mut Self {
				   self.#name = value;
				   self
			  }
			  pub fn #field_getter_fn(&self) -> &#field_ty {
				   &self.#name
			  }
		});
	}

	resource_item.attrs.push(syn::parse_quote!(
	  #[derive(
			Default, Debug, Clone, PartialEq, ::mashin_sdk::ext::serde::Deserialize
	  )]
	));

	resource_item.fields.iter_mut().for_each(|field| {
		field.attrs.push(syn::parse_quote! {
			  #[serde(deserialize_with = "::mashin_sdk::deserialize_state_field", default)]
		})
	});

	["__urn", "__config", "__name"].iter().for_each(|item| {
		let ident = quote::format_ident!("{}", item);
		let serializer = quote! {
			  state.serialize_field(#item,
				   &::mashin_sdk::ext::serde_json::json! {
						{
							  "__value": self.#ident,
							  "__sensitive": false,
						}
				   }
			  )?;
		};
		fields_json.push(serializer);
	});

	let total_fields = fields_json.len();
	let vis = &resource_item.vis;
	let fields = resource_item.fields.iter().collect::<Vec<_>>();
	let config_ident = &def.config.ident;
	let docs = &resource.docs;

	quote::quote_spanned! { def.resource.attr_span =>
	  #vis use #isolated_ident::#resource_ident;
	  mod #isolated_ident {
			use super::*;

			#[derive(Default, Debug, Clone, PartialEq, ::mashin_sdk::ext::serde::Deserialize)]
			#[serde(rename_all = "camelCase")]
			#( #[doc = #docs] )*
			#vis struct #resource_ident {
				 #(#fields,)*
				 #[serde(deserialize_with = "::mashin_sdk::deserialize_state_field", default)]
				 #[serde(rename = "__config")]
				 __config: #config_ident,
				 #[serde(deserialize_with = "::mashin_sdk::deserialize_state_field", default)]
				 #[serde(rename = "__name")]
				 __name: String,
				 #[serde(deserialize_with = "::mashin_sdk::deserialize_state_field", default)]
				 #[serde(rename = "__urn")]
				 __urn: String,
			}

			impl #resource_ident {
				 pub fn config(&self) -> &#config_ident {
					  &self.__config
				 }

				 #( #fields_helpers_impl )*
			}

			impl serde::Serialize for #resource_ident {
				 fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
				 where
					  S: serde::Serializer,
				 {
					  use ::serde::ser::SerializeStruct as _;
					  let mut state = serializer.serialize_struct(#resource_name_str, #total_fields)?;
					  #( #fields_json )*
					  state.end()
				 }
			}

			impl ResourceDefault for #resource_ident {
				 fn new(name: &str, urn: &str) -> Self
				 where
					  Self: Sized {
							Self {
								 __name: name.to_string(),
								 __urn: urn.to_string(),
								 ..Default::default()
							}
					  }

				 fn set_raw_config(&mut self, config: &::std::rc::Rc<::mashin_sdk::ext::serde_json::Value>) {
					  let config = config.as_ref().clone();
					  self.__config = ::mashin_sdk::ext::serde_json::from_value::<#config_ident>(config).unwrap_or_default();
				 }

				 fn name(&self) -> &str {
					  self.__name.as_str()
				 }

				 fn urn(&self) -> &str {
					  self.__urn.as_str()
				 }
			}

			#[cfg(test)]
			mod #mod_ident {
				 use super::*;

				 #[test]
				 fn build_successfully() -> () {
					  let resource = #resource_ident::default();
					  assert_eq!(resource.name(), "");
					  assert_eq!(resource.urn(), "");
					  assert_eq!(resource.config().clone(), #config_ident::default());
				 }
			}
	  }
	}
}
