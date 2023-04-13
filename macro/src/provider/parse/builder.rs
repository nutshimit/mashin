use darling::ToTokens;
use syn::spanned::Spanned;

/// Input definition for the pallet builder.
#[derive(Debug)]
pub struct BuilderDef {
    pub index: usize,
    /// The span of the pallet::builder attribute.
    pub attr_span: proc_macro2::Span,
}

mod keyword {
    syn::custom_keyword!(builder);
}

impl BuilderDef {
    pub fn try_from(
        attr_span: proc_macro2::Span,
        index: usize,
        item: &mut syn::Item,
    ) -> syn::Result<Self> {
        let item = if let syn::Item::Impl(item) = item {
            item
        } else {
            let msg = "Invalid mashin::builder, expected struct";
            return Err(syn::Error::new(item.span(), msg));
        };

        Ok(Self { index, attr_span })
    }
}
