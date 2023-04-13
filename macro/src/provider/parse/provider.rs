use darling::ToTokens;
use syn::spanned::Spanned;

#[derive(Debug)]
pub struct ProviderDef {
    pub index: usize,
    pub attr_span: proc_macro2::Span,
}

mod keyword {
    syn::custom_keyword!(Provider);
}

impl ProviderDef {
    pub fn try_from(
        attr_span: proc_macro2::Span,
        index: usize,
        item: &mut syn::Item,
    ) -> syn::Result<Self> {
        let item = if let syn::Item::Struct(item) = item {
            item
        } else {
            let msg = "Invalid provider::provider, expected struct";
            return Err(syn::Error::new(item.span(), msg));
        };

        if !matches!(item.vis, syn::Visibility::Public(_)) {
            let msg = "Invalid provider::provider, struct must be public";
            return Err(syn::Error::new(item.span(), msg));
        }

        syn::parse2::<keyword::Provider>(item.ident.to_token_stream())?;

        Ok(Self { index, attr_span })
    }
}
