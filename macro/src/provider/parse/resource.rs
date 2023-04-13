use darling::ToTokens;
use syn::spanned::Spanned;

use super::helper;

#[derive(Debug)]
pub struct ResourceDef {
    pub name: String,
    pub config: syn::Ident,
    pub index: usize,
    pub attr_span: proc_macro2::Span,
    pub ident: syn::Ident,
}

mod keyword {
    syn::custom_keyword!(sensitive);
    syn::custom_keyword!(mashin);
}

impl ResourceDef {
    pub fn try_from(
        name: String,
        config: syn::Ident,
        attr_span: proc_macro2::Span,
        index: usize,
        item: &mut syn::Item,
    ) -> syn::Result<Self> {
        let item = if let syn::Item::Struct(item) = item {
            item
        } else {
            let msg = "Invalid provider::resource, expected struct";
            return Err(syn::Error::new(item.span(), msg));
        };

        let ident = item.ident.clone();
        if !matches!(item.vis, syn::Visibility::Public(_)) {
            let msg = "Invalid provider::resource, struct must be public";
            return Err(syn::Error::new(item.span(), msg));
        }

        Ok(Self {
            name,
            config,
            attr_span,
            index,
            ident,
        })
    }
}

/// Input definition for the pallet builder.
#[derive(Debug)]
pub struct ResourceImplDef {
    pub index: usize,
    /// The span of the pallet::builder attribute.
    pub attr_span: proc_macro2::Span,
}

impl ResourceImplDef {
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

#[derive(Debug)]
pub struct ResourceConfigDef {
    pub index: usize,
    /// The span of the pallet::builder attribute.
    pub attr_span: proc_macro2::Span,
}

impl ResourceConfigDef {
    pub fn try_from(
        attr_span: proc_macro2::Span,
        index: usize,
        item: &mut syn::Item,
    ) -> syn::Result<Self> {
        let item = if let syn::Item::Struct(item) = item {
            item
        } else {
            let msg = "Invalid mashin::builder, expected struct";
            return Err(syn::Error::new(item.span(), msg));
        };

        Ok(Self { index, attr_span })
    }
}
