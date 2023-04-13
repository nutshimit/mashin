use darling::ToTokens;
use syn::spanned::Spanned;

/// Input definition for the pallet state.
#[derive(Debug)]
pub struct StateDef {
    pub index: usize,
    /// The span of the pallet::state attribute.
    pub attr_span: proc_macro2::Span,
    pub ident: syn::Ident,
}

mod keyword {
    syn::custom_keyword!(State);
}

impl StateDef {
    pub fn try_from(
        attr_span: proc_macro2::Span,
        index: usize,
        item: &mut syn::Item,
    ) -> syn::Result<Self> {
        let item = if let syn::Item::Struct(item) = item {
            item
        } else {
            let msg = "Invalid mashin::state, expected struct";
            return Err(syn::Error::new(item.span(), msg));
        };

        if !matches!(item.vis, syn::Visibility::Public(_)) {
            let msg = "Invalid mashin::state, struct must be public";
            return Err(syn::Error::new(item.span(), msg));
        }

        syn::parse2::<keyword::State>(item.ident.to_token_stream())?;

        Ok(Self {
            index,
            attr_span,
            ident: item.ident.clone(),
        })
    }
}
