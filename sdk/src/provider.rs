#[macro_export]
macro_rules! construct_provider {
	(
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
