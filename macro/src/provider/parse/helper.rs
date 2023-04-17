use darling::ToTokens;

pub trait MutItemAttrs {
	fn mut_item_attrs(&mut self) -> Option<&mut Vec<syn::Attribute>>;
}

/// Take all the pallet attributes (e.g. attribute like `#[provider..]`) and decode them to `Attr`
pub fn _take_item_provider_attrs<Attr>(item: &mut impl MutItemAttrs) -> syn::Result<Vec<Attr>>
where
	Attr: syn::parse::Parse,
{
	let mut pallet_attrs = Vec::new();

	while let Some(attr) = take_first_item_provider_attr(item)? {
		pallet_attrs.push(attr)
	}

	Ok(pallet_attrs)
}

/// Take the first pallet attribute (e.g. attribute like `#[provider..]`) and decode it to `Attr`
pub fn take_first_item_provider_attr<Attr>(
	item: &mut impl MutItemAttrs,
) -> syn::Result<Option<Attr>>
where
	Attr: syn::parse::Parse,
{
	let attrs = if let Some(attrs) = item.mut_item_attrs() {
		attrs
	} else {
		return Ok(None)
	};

	if let Some(index) = attrs.iter().position(|attr| {
		attr.path.segments.first().map_or(false, |segment| segment.ident == "mashin")
	}) {
		let pallet_attr = attrs.remove(index);
		Ok(Some(syn::parse2(pallet_attr.into_token_stream())?))
	} else {
		Ok(None)
	}
}

impl MutItemAttrs for syn::Item {
	fn mut_item_attrs(&mut self) -> Option<&mut Vec<syn::Attribute>> {
		match self {
			Self::Const(item) => Some(item.attrs.as_mut()),
			Self::Enum(item) => Some(item.attrs.as_mut()),
			Self::ExternCrate(item) => Some(item.attrs.as_mut()),
			Self::Fn(item) => Some(item.attrs.as_mut()),
			Self::ForeignMod(item) => Some(item.attrs.as_mut()),
			Self::Impl(item) => Some(item.attrs.as_mut()),
			Self::Macro(item) => Some(item.attrs.as_mut()),
			Self::Macro2(item) => Some(item.attrs.as_mut()),
			Self::Mod(item) => Some(item.attrs.as_mut()),
			Self::Static(item) => Some(item.attrs.as_mut()),
			Self::Struct(item) => Some(item.attrs.as_mut()),
			Self::Trait(item) => Some(item.attrs.as_mut()),
			Self::TraitAlias(item) => Some(item.attrs.as_mut()),
			Self::Type(item) => Some(item.attrs.as_mut()),
			Self::Union(item) => Some(item.attrs.as_mut()),
			Self::Use(item) => Some(item.attrs.as_mut()),
			_ => None,
		}
	}
}

impl MutItemAttrs for syn::TraitItem {
	fn mut_item_attrs(&mut self) -> Option<&mut Vec<syn::Attribute>> {
		match self {
			Self::Const(item) => Some(item.attrs.as_mut()),
			Self::Method(item) => Some(item.attrs.as_mut()),
			Self::Type(item) => Some(item.attrs.as_mut()),
			Self::Macro(item) => Some(item.attrs.as_mut()),
			_ => None,
		}
	}
}

impl MutItemAttrs for Vec<syn::Attribute> {
	fn mut_item_attrs(&mut self) -> Option<&mut Vec<syn::Attribute>> {
		Some(self)
	}
}

impl MutItemAttrs for syn::ItemMod {
	fn mut_item_attrs(&mut self) -> Option<&mut Vec<syn::Attribute>> {
		Some(&mut self.attrs)
	}
}

impl MutItemAttrs for syn::ImplItemMethod {
	fn mut_item_attrs(&mut self) -> Option<&mut Vec<syn::Attribute>> {
		Some(&mut self.attrs)
	}
}
impl MutItemAttrs for syn::ImplItem {
	fn mut_item_attrs(&mut self) -> Option<&mut Vec<syn::Attribute>> {
		match self {
			syn::ImplItem::Const(i) => Some(i.attrs.as_mut()),
			syn::ImplItem::Method(i) => Some(i.attrs.as_mut()),
			syn::ImplItem::Type(i) => Some(i.attrs.as_mut()),
			syn::ImplItem::Macro(i) => Some(i.attrs.as_mut()),
			_ => None,
		}
	}
}
