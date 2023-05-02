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
 *  This file is licensed as MIT. See LICENSE for details.  *
 *                                                          *
\* ---------------------------------------------------------*/

use crate::provider::parse::Def;
use syn::Item;

pub fn expand_resources(def: &mut Def) -> proc_macro2::TokenStream {
	let _item = {
		let item =
			&mut def.item.content.as_mut().expect("Checked by def parser").1[def.resources.index];
		let item_cloned = item.clone();
		*item = Item::Verbatim(quote::quote!());

		if let syn::Item::Enum(item) = item_cloned {
			item
		} else {
			unreachable!("Checked by config parser")
		}
	};

	quote::quote! {}
}
