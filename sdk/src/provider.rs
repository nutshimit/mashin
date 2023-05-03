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

/// The `construct_provider!` macro accepts the following parameters:
///
/// - `$name`:     The name of the provider.
///                Choose a name that reflects the providers purpose, such as the API or service it interacts
///                with. The name should be in **snake_case** and must be unique among all providers in the ecosystem.
///
/// - `config`:    An optional configuration struct for the provider. This configuration
///                struct serves as a way to define the schema for the provider's configuration
///                options. The fields in this struct will be exposed in the MashinScript (Typescript)
///                and the corresponding types will be generated automatically, enabling users to configure
///                the provider with a strongly-typed and validated schema. The configuration provided by the
///                user in MashinScript will be used in the state function to create and initialize the
///                provider based on the desired configuration. This ensures a seamless and type-safe integration
///                between the Rust and Typescript code while maintaining the flexibility to accommodate different
///                configurations.
///                Additionally, all comments on fields within the config struct, will be automatically
///                exported to the Typescript bindings, providing helpful context and guidance to users
///                who interact with the provider options in the Typescript environment.
///
/// - `resources`: A list of resources associated with the provider. This list contains the module paths of
///                resources created using the `#[mashin_sdk::resource]` attribute. By specifying these resources,
///                you establish a clear link between the provider and its associated resources. A provider can
///                include multiple resources, enabling it to manage a diverse range of infrastructure components.
///                Each provider should include at least one resource to ensure its functionality and purpose within
///                the infrastructure management ecosystem.
///
/// - `state`:     An optional function to initialize the provider state. The state function is executed when the provider
///                is created and subsequently passed to all resources associated with the provider. This function receives
///                the config parameter, which includes values from the Typescript environment defined by the user, allowing
///                the provider and its associated client to be configured accordingly. By initializing components such as API
///                services, establishing connections, and performing validations within the state function and storing the
///                resulting client in the provider state, resources can access the provider_state during their CRUD operations.
///                This approach enables efficient resource management and reduces the need to re-establish connections or perform
///                redundant validations.
///
/// - `on_drop`:   An optional function to be called when the provider is dropped.
///
/// This macro provides a convenient way for developers to define and construct providers
/// for the Mashin engine. It takes care of setting up the provider's configuration, resources,
/// state, and optional on_drop function. By using this macro, developers can focus on implementing
/// their provider's functionality without worrying about the underlying boilerplate code.
#[macro_export]
macro_rules! construct_provider {
	(
	 $(#[$provider_attr:meta])*
	 $name:ident
    $(, config = { $( $(#[$configs_attr:meta])* $configs_id:ident : $configs_type:ty ),* $(,)? } )?
    $(, resources = [ $( $(#[$resource_attr:meta])* $resource:ident ),* $(,)? ] )?
    $(, state = $state_fn:expr )?
    $(, on_drop = $drop_fn:expr )?
    $(,)?
  ) => {
		#[mashin_sdk::provider]
		pub mod $name {
         use super::*;
			use std::sync::Arc;
			use mashin_sdk::{
				ext::parking_lot::Mutex, ProviderBuilder, ProviderDefault, ProviderState,
				ResourceDefault, ResourceDiff, Result,
			};

			$(#[$provider_attr])*
			#[mashin::provider]
			pub struct Provider;

			#[mashin::resource]
			pub enum Resources {
				$(
              $(
              	$(#[$resource_attr])*
              	$resource
              ),*
				)?
			}

			#[mashin::config]
			pub struct Config {
            $(
               $(
                  $(#[$configs_attr])*
                  $configs_id: $configs_type
               ),*
            )?
			}

			#[mashin::builder]
			impl ProviderBuilder for Provider {
				async fn build(&mut self) -> mashin_sdk::Result<()> {
               $(
                  let state_fn: fn(::std::sync::Arc<::mashin_sdk::ext::parking_lot::Mutex<::mashin_sdk::ProviderState>>, &Config) = $state_fn;
                  state_fn(self.state(), self.config());
               )?;
              Ok(())
				}
			}

			impl Drop for Provider {
				fn drop(&mut self) {
               $(
                  let drop_fn: fn(&mut Provider) = $drop_fn;
                  drop_fn(self);
               )?;
				}
			}
		}
	};
}
