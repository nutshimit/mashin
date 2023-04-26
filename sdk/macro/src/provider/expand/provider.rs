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
 *   see LICENSE for license details.                       *
 *                                                          *
\* ---------------------------------------------------------*/

use crate::provider::parse::Def;
use inflector::Inflector;
use quote::format_ident;
use std::env;
use syn::Item;

pub fn expand_provider(def: &mut Def) -> proc_macro2::TokenStream {
	let mod_ident = &def.item.ident;
	let provider_item = {
		let provider = &def.provider;
		let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[provider.index];
		let item_cloned = item.clone();
		*item = Item::Verbatim(quote::quote!());
		if let syn::Item::Struct(item) = item_cloned {
			item
		} else {
			unreachable!("Checked by config parser")
		}
	};

	let resource_item = {
		let resource = &def.resources;
		let item = &mut def.item.content.as_mut().expect("Checked by def parser").1[resource.index];
		if let syn::Item::Enum(item) = item {
			item
		} else {
			unreachable!("Checked by config parser")
		}
	};

	let config = &def.config.ident;
	let vis = &provider_item.vis;
	let ident = &provider_item.ident;
	let isolated_ident = format_ident!("{}", ident.to_string().to_snake_case());

	let fields = provider_item.fields.iter().collect::<Vec<_>>();
	let provider_name = match env::var("MASHIN_PKG_NAME") {
		Ok(version) => version,
		Err(_e) => env!("CARGO_PKG_NAME").to_string(),
	};

	let resources_map = resource_item.variants.iter().map(|resource| {
		let name = &resource.ident.to_string();
		let resource = &resource.ident;
		quote::quote! {
			#name => Ok(#resource::Resource::from_current_state(
				name,
				&urn.to_string(),
				state.clone(),
			)?),
		}
	});

	let docs = &def.provider.docs;

	quote::quote_spanned! { def.provider.attr_span =>
		#vis use #isolated_ident::#ident;

		mod #isolated_ident {
			use super::*;

			#[derive(Default)]
			#( #[doc = #docs] )*
			#vis struct #ident {
				#(#fields,)*
				__config: #config,
				__state: ::std::sync::Arc<::mashin_sdk::ext::parking_lot::Mutex<::mashin_sdk::ProviderState>>,
			}

			impl #ident {
				pub fn config(&self) -> &#config {
					&self.__config
				}

				pub fn update_config(&mut self, config: ::mashin_sdk::ext::serde_json::Value) -> ::mashin_sdk::Result<()> {
					self.__config = ::mashin_sdk::ext::serde_json::from_value(config)?;
					Ok(())
				}
			}

			impl mashin_sdk::ProviderDefault for #ident {
				fn state(&mut self) -> ::std::sync::Arc<::mashin_sdk::ext::parking_lot::Mutex<::mashin_sdk::ProviderState>> {
					self.__state.clone()
				}

				fn build_resource(
					&self,
					urn: &::std::rc::Rc<::mashin_sdk::Urn>,
					state: &::std::rc::Rc<::std::cell::RefCell<::mashin_sdk::ext::serde_json::Value>>,
				) -> ::mashin_sdk::Result<::std::rc::Rc<::std::cell::RefCell<dyn ::mashin_sdk::Resource>>> {
					let raw_urn = urn.nss().split(':').collect::<Vec<_>>()[1..].join(":");
					let module_urn = raw_urn.to_lowercase();

					// resource name
					let name = urn
						.q_component()
						.ok_or(::mashin_sdk::ext::anyhow::anyhow!("expect valid urn (name not found)"))?;

					match module_urn.as_str() {
						#( #resources_map )*
						_ => ::mashin_sdk::ext::anyhow::bail!("invalid URN"),
					}
				}
			}

			impl mashin_sdk::Provider for #ident {}

			#[cfg(test)]
			mod #mod_ident {
				use super::*;
				use ::mashin_sdk::ext::tokio;
				use ::mashin_sdk::ProviderBuilder;

				#[tokio::test]
				async fn build_successfully() -> Result<()> {
					let mut provider = #ident::default();
					provider.build().await
				}
			}
		}

		#[no_mangle]
		pub extern "C" fn new<'sym>(
			logger_ptr: *mut &'static ::mashin_sdk::CliLogger,
			args_ptr: *const u8,
			args_length: usize,
		) -> *mut #ident {
			let buf = unsafe { ::std::slice::from_raw_parts(args_ptr, args_length) };
			let args = ::mashin_sdk::ext::serde_json::from_slice(buf).expect("valid buffer for `new` args");

			__MASHIN_LOG_INIT.call_once(|| {
				let logger = unsafe {
					Box::from_raw(logger_ptr)
				};

				// FIXME: pass it from the cli level
				::log::set_max_level(log::LevelFilter::Info);
				::log::set_boxed_logger(Box::new(logger)).expect("valid logger");
				setup_panic_hook();
			});

			fn __execute(args: ::mashin_sdk::ext::serde_json::Value) -> #ident {
				let runtime = ::mashin_sdk::ext::tokio::runtime::Runtime::new().expect("New runtime");
				let mut provider = #ident::default();
				provider.update_config(args).expect("valid config");
				runtime.block_on(provider.build()).expect("valid provider");
				provider
			}

			let result = __execute(args);
			Box::into_raw(Box::new(result))
		}

		#[no_mangle]
		pub extern "C" fn run<'sym>(
			handle_ptr: *mut #ident,
			args_ptr: *const u8,
			args_length: usize,
		) -> *const u8 {
			assert!(!handle_ptr.is_null());
			let buf = unsafe { ::std::slice::from_raw_parts(args_ptr, args_length) };
			let args = ::mashin_sdk::ext::serde_json::from_slice(buf).expect("valid buffer");
			let provider = unsafe { &mut *handle_ptr };

			fn __execute(
				provider: &mut dyn mashin_sdk::Provider,
				args: ::mashin_sdk::ResourceArgs
			) -> Vec<u8> {
				let runtime = ::mashin_sdk::ext::tokio::runtime::Runtime::new().expect("New runtime");
				let provider_state = provider.state();
				let urn = &args.urn;
				let raw_config = &args.raw_config.clone();
				let raw_state = &args.raw_state.clone();

				let resource = provider
					.build_resource(urn, raw_state)
					.expect("valid raw state");

				let mut resource = resource.borrow_mut();

				// grab the state before applying our values
				resource.set_raw_config(raw_config);

				runtime
					.block_on(async {
						match args.action.as_ref() {
							::mashin_sdk::ResourceAction::Update { diff } => resource.update(provider_state, diff),
							::mashin_sdk::ResourceAction::Create => resource.create(provider_state),
							::mashin_sdk::ResourceAction::Delete => resource.delete(provider_state),
							::mashin_sdk::ResourceAction::Get => resource.get(provider_state),
						}
						.await
					})
					.expect("valid execution");

				let state = resource.to_raw_state().expect("valid resource");
				let result = ::mashin_sdk::ResourceResult::new(state);
				let json = ::mashin_sdk::ext::serde_json::to_string(&result).expect("valid `ResourceResult`");
				let encoded = json.into_bytes();
				let length = (encoded.len() as u32).to_be_bytes();
				let mut v = length.to_vec();
				v.extend(encoded.clone());
				v
			}

			let result = __execute(provider, args);
			let ret = result.as_ptr();
			::std::mem::forget(result);
			ret
		}

		#[no_mangle]
		pub extern "C" fn drop<'sym>(handle: *mut #ident) {
			assert!(!handle.is_null());
			unsafe {
				std::ptr::drop_in_place(handle);
				std::alloc::dealloc(handle as *mut u8, ::std::alloc::Layout::new::<#ident>());
			}
		}

		fn setup_panic_hook() {
			let orig_hook = std::panic::take_hook();
			std::panic::set_hook(Box::new(move |panic_info| {
				eprintln!("\n============================================================");
				eprintln!("Mashin has panicked. This is a bug in Mashin. Please report this");
				eprintln!("at https://github.com/nutshimit/mashin/issues/new.");
				eprintln!("If you can reliably reproduce this panic, include the");
				eprintln!("reproduction steps and re-run with the RUST_BACKTRACE=1 env");
				eprintln!("var set and include the backtrace in your report.");
				eprintln!();
				eprintln!("Platform: {} {}", std::env::consts::OS, std::env::consts::ARCH);
				eprintln!("Version: {}", std::env!("CARGO_PKG_VERSION"));
				eprintln!("Provider: {}", #provider_name);
				eprintln!("Args: {:?}", std::env::args().collect::<Vec<_>>());
				eprintln!();
				orig_hook(panic_info);
				std::process::exit(1);
			}));
		}

	}
}
