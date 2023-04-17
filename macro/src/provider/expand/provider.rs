use crate::provider::parse::Def;
use std::env;
use syn::Item;

pub fn expand_provider(def: &mut Def) -> proc_macro2::TokenStream {
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

	let config = &def.config.ident;
	let vis = &provider_item.vis;
	let ident = &provider_item.ident;

	let fields = provider_item.fields.iter().collect::<Vec<_>>();
	let provider_target = format!(
		"mashin::provider::{}",
		match env::var("MASHIN_PKG_NAME") {
			Ok(version) => version,
			Err(_e) => env!("CARGO_PKG_NAME").to_string(),
		}
	);

	let resources_map = def.resources.iter().map(|resource| {
		let name = resource.name.clone();
		let resource = &resource.ident;
		quote::quote! {
			#name => Ok(#resource::from_current_state(
				name,
				&urn.to_string(),
				state.clone(),
			)?),
		}
	});

	quote::quote_spanned! { def.provider.attr_span =>
		macro_rules! log {
			($level:tt,  $patter:expr $(, $values:expr)* $(,)?) => {
				::log::$level!(
					target: #provider_target,
					$patter  $(, $values)*
				)
			};
		}
		pub(super) use log;

		#[derive(Default)]
		#vis struct #ident {
			#(#fields,)*
			__config: #config,
			__state: Box<::mashin_sdk::ProviderState>,
		}

		impl #ident {
			pub fn config(&self) -> &#config {
				&self.__config
			}

			pub fn update_config(&mut self, config: ::std::rc::Rc<::mashin_sdk::ext::serde_json::Value>) -> ::mashin_sdk::Result<()> {
				self.__config = ::mashin_sdk::ext::serde_json::from_value(config.as_ref().clone())?;
				Ok(())
			}
		}

		impl mashin_sdk::ProviderDefault for #ident {
			fn state_as_ref(&self) -> &mashin_sdk::ProviderState {
				self.__state.as_ref()
			}

			fn state(&mut self) -> &mut Box<::mashin_sdk::ProviderState> {
				&mut self.__state
			}

			fn __from_current_state(
				&self,
				urn: &::std::rc::Rc<::mashin_sdk::Urn>,
				state: &::std::rc::Rc<::std::cell::RefCell<::mashin_sdk::ext::serde_json::Value>>,
			) -> ::mashin_sdk::Result<::std::rc::Rc<::std::cell::RefCell<dyn ::mashin_sdk::Resource>>> {
				let raw_urn = urn.nss().split(':').collect::<Vec<_>>()[1..].join(":");
				// expect; s3:bucket
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
		mod __provider {
			use super::*;
			use ::mashin_sdk::ext::tokio;
			use ::mashin_sdk::ProviderBuilder;

			#[tokio::test]
			async fn build_successfully() -> Result<()> {
				let mut provider = #ident::default();
				provider.build().await
			}
		}

		#[no_mangle]
		pub extern "C" fn new(
			logger_ptr: *mut &'static ::mashin_sdk::CliLogger,
			args_ptr: *mut ::mashin_sdk::ext::serde_json::Value) -> *mut #ident {
			__MASHIN_LOG_INIT.call_once(|| {
				let logger = unsafe {
					Box::from_raw(logger_ptr)
				};

				// FIXME: pass it from the cli level
				::log::set_max_level(log::LevelFilter::Info);
				::log::set_boxed_logger(Box::new(logger)).expect("valid logger");
				setup_panic_hook();
			});

			let args = unsafe { ::std::rc::Rc::from_raw(args_ptr) };

			let runtime = ::mashin_sdk::ext::tokio::runtime::Runtime::new().expect("New runtime");
			let mut provider = #ident::default();
			provider.update_config(args).expect("valid config");
			runtime.block_on(provider.build()).expect("valid provider");
			let static_ref = Box::new(provider);


			Box::into_raw(static_ref)
		}

		#[no_mangle]
		pub extern "C" fn run(
			handle_ptr: *mut #ident,
			args_ptr: *mut ::mashin_sdk::ResourceArgs,
		) -> *mut ::mashin_sdk::ResourceResult {

			let runtime = ::mashin_sdk::ext::tokio::runtime::Runtime::new().expect("New runtime");

			// grab current provider
			assert!(!handle_ptr.is_null());
			let provider = unsafe { &mut *handle_ptr };
			let provider_state = provider.state_as_ref();

			// resource URN
			let args = unsafe { ::std::rc::Rc::from_raw(args_ptr) };

			let urn = &args.urn;
			let raw_config = &args.raw_config.clone();
			let raw_state = &args.raw_state.clone();

			let resource = provider
				.__from_current_state(urn, raw_state)
				.expect("Valid resource");

			let mut resource = resource.borrow_mut();

			// grab the state before applying our values
			resource.__set_config_from_value(raw_config);

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

			::std::rc::Rc::into_raw(::std::rc::Rc::new(result)) as *mut ::mashin_sdk::ResourceResult
		}

		#[no_mangle]
		pub extern "C" fn drop(handle: *mut #ident) {
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
				eprintln!("Args: {:?}", std::env::args().collect::<Vec<_>>());
				eprintln!();
				orig_hook(panic_info);
				std::process::exit(1);
			}));
		}

	}
}
