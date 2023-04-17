use crate::provider::{
	expand::helper::process_struct,
	parse::{Def, InternalMashinType},
};
use quote::{format_ident, quote};
use syn::{Item, Meta};

pub fn expand_resources(def: &mut Def) -> proc_macro2::TokenStream {
	// process all resources, need to be done before we overwrite the output
	for res in def.resources.clone() {
		process_struct(def, res.index, InternalMashinType::Resource(res)).expect("valid ts");
	}

	let resources = def.resources.iter().map(|resource| {
        let mut resource_item = {

                let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[resource.index];
                let item_cloned = item.clone();
                *item = Item::Verbatim(quote::quote!());
            if let syn::Item::Struct(item) = item_cloned {
                item
            } else {
                unreachable!("Checked by config parser")
            }
        };

        let resource_ident = &resource_item.ident;
        let resource_name_str = resource_item.ident.to_string();

        let mut fields_json = Vec::new();

        for field in resource_item.fields.iter_mut() {
            let name = &field.ident.clone().expect("valid name");

            let mut sensitive = false;
            let mut attr_id = 0;

            for attribute in field.attrs.clone() {
                if let Ok(Meta::Path(path)) = attribute.parse_meta() {
                    if path.is_ident("sensitive") {
                        sensitive = true;
                        field.attrs.remove(attr_id);
                    }
                    attr_id += 1;
                }
            }


            let field_name = name.to_string();
            let field_json = quote! {
                &::mashin_sdk::ext::serde_json::json! {
                    {
                        "__value": self.#name,
                        "__sensitive": #sensitive,
                    }
                }
            };

            fields_json.push(quote! { state.serialize_field(#field_name, #field_json)?; });
        }

        resource_item.attrs.push(syn::parse_quote!(
            #[derive(
                Default, Debug, Clone, PartialEq, ::mashin_sdk::ext::serde::Deserialize
            )]
        ));

        resource_item
            .fields
            .iter_mut()
            .for_each(|field| field.attrs.push(syn::parse_quote!{
                #[serde(deserialize_with = "::mashin_sdk::deserialize_state_field", default)]
            }));


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
        let config_ident = &resource.config;
        let test_ident = format_ident!("__{}", resource_ident.to_string().to_lowercase());

        quote! {

          #[derive(Default, Debug, Clone, PartialEq, ::mashin_sdk::ext::serde::Deserialize)]
          #[serde(rename_all = "camelCase")]
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
                fn __default_with_params(name: &str, urn: &str) -> Self
                where
                    Self: Sized {
                        Self {
                            __name: name.to_string(),
                            __urn: urn.to_string(),
                            ..Default::default()
                        }
                }

                fn __set_config_from_value(&mut self, config: &::std::rc::Rc<::mashin_sdk::ext::serde_json::Value>) {
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
           mod #test_ident {
               use super::*;
               use ::mashin_sdk::ext::tokio;
               use ::mashin_sdk::ProviderBuilder;

               #[test]
               fn build_successfully() -> () {
                   let resource = #resource_ident::default();
                   assert_eq!(resource.__name, "");
                   assert_eq!(resource.__urn, "");
                   assert_eq!(resource.__config, #config_ident::default());
               }
           }
        }
    }).collect::<Vec<_>>();

	quote::quote! {
		#( #resources )*
	}
}
