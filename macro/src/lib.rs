use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Meta};

mod provider;

#[proc_macro_attribute]
pub fn provider(attr: TokenStream, item: TokenStream) -> TokenStream {
    provider::provider(attr, item)
}

#[proc_macro_attribute]
pub fn resource(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);

    let name = &input.ident;
    let vis = &input.vis;
    let attrs = input.attrs;

    let data = match input.data {
        Data::Struct(data) => data,
        _ => panic!("Resource macro can only be used on a struct"),
    };

    let mut fields_json = Vec::new();
    let mut fields_raw = Vec::new();

    for field in data.fields.iter() {
        let name = &field.ident.clone().expect("valid name");
        let ty = field.ty.clone();

        let sensitive = field.attrs.iter().any(|attr| {
            if let Ok(Meta::Path(path)) = attr.parse_meta() {
                path.is_ident("sensitive")
            } else {
                false
            }
        });

        let field_name = name.to_string();
        let field_json = quote! {
            &::mashin_sdk::ext::serde_json::json! {
                {
                    "__value": self.#name.clone(),
                    "__sensitive": #sensitive,
                }
            }
        };

        fields_json.push(quote! { state.serialize_field(#field_name, #field_json)?; });
        fields_raw.push(quote! {
            #[serde(deserialize_with = "::mashin_sdk::deserialize_state_field", default)]
            #name: #ty,
        });
    }

    let total_fields = fields_raw.len();
    let struct_name_str = name.to_string();

    let expanded = quote! {
      #( #attrs )*
      #[derive(Default, Debug, Clone, PartialEq, ::mashin_sdk::ext::serde::Deserialize)]
      #vis struct #name {
         #( #fields_raw )*
      }

      impl serde::Serialize for #name {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut state = serializer.serialize_struct(#struct_name_str, #total_fields)?;
            #( #fields_json )*
            state.end()
        }
      }

    };

    TokenStream::from(expanded)
}
