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

use proc_macro::TokenStream;

mod provider;
mod resource;
mod utils;

/// This proc macro is used internally by the `construct_provider!` macro and should not be used directly by users.
/// This macro generates the necessary code for creating a provider, including the provider struct, configuration,
/// resources link, and state management. It also handles the provider's lifecycle, such as the initialization and
/// the drop operation.
///
/// When the provider macro is invoked by the `construct_provider!` macro, it generates the following components:
///
/// 1. **Provider struct**:      A struct representing the provider, which stores the provider's state and configuration.
///
/// 2. **Configuration**:        A struct that contains the provider's configuration fields, which can be set by users in
///                              the MashinScript environment.
///
/// 3. **Resources**:            An enum that contains the associated resources, which are defined using the `#[mashin_sdk::resource]`
///                              macro and linked to the provider.
///
/// 4. **State management**:     Functions for initializing the provider's state, as well as any custom state management logic specified
///                              by the user.
///
/// 5. **Lifecycle management**: Implementation of the Drop trait for the provider, which allows for custom cleanup logic when the provider
///                              is dropped.
///
/// 6. **"C" functions**:        Generates the necessary `"C" functions` for the cdylib, including `new`, `run`, and `drop`. These functions
///                              are used for creating a new provider, running the provider with specified arguments, and dropping the provider,
///                              respectively.
///
/// **Note that the provider macro should not be used directly by users. Instead, use the `construct_provider!` macro to define your providers
/// and let it handle the invocation of the provider macro internally.**
///
#[proc_macro_attribute]
pub fn provider(attr: TokenStream, item: TokenStream) -> TokenStream {
	provider::provider(attr, item)
}

/// This proc macro is used to define a resource.
///
/// When creating a resource with this macro, it is important to choose a descriptive and meaningful name.
///
/// The name should be concise, easy to understand, and reflect the purpose of the resource. Follow the guidelines
/// below to ensure you select the appropriate name for your resource:
///
/// 1.   **Use snake_case**: In Rust, it is conventional to use snake_case (all lowercase letters separated by underscores)
///      for naming modules. Stick to this convention when naming your resource.
///
/// 2.   **Be descriptive**: Choose a name that accurately reflects the purpose and functionality of the resource.
///      It should provide a clear indication of what the resource does or represents.
///
/// 3.   **Keep it concise**: While being descriptive is important, avoid using overly long names. Aim for a balance between
///      descriptiveness and brevity to ensure the name is easy to read and understand.
///
/// 4.   **Avoid using reserved words**: Make sure not to use any Rust reserved words or any words that could cause confusion
///      with other parts of the Mashin ecosystem.
///
/// Here's an example of how to correctly name your resource:
/// ```no_run
/// #[mashin_sdk::resource]
/// pub mod s3_bucket {
///   // Resource implementation goes here
/// }
/// ```
///
/// Within the module, several attributes can be utilized.
///
/// `#[mashin::config]`:   This attribute is used to generate the configuration schema for the resource.
///                        Similar to the provider config, the schema will be exposed in the Typescript
///                        environment, allowing users to configure the resource based on the struct defined
///                        by the developer. The Typescript bindings are automatically generated, and the
///                        resource config can be accessed within the CRUD operations using `self.config()`.
///
/// `#[mashin::resource]`: This attribute is where the resource schema is defined. To avoid exporting specific
///                        fields to the Typescript environment, use the `#[sensitive]` attribute on those fields.
///                        As a result, sensitive data will only exist within the encrypted state and will not be
///                        accessible in the Typescript environment. Typescript bindings are automatically generated
///                        for all fields, excluding sensitive ones.
///
/// `#[mashin::calls]`:    This attribute is used to define the CRUD operations. Developers can implement the required
///                        methods for creating, reading, updating, and deleting resources.
///
/// `#[mashin::ts]`:       If an external struct is used within a resource or its configuration, this attribute can be
///                        added to generate bindings for that struct as well. This can be useful in various scenarios
///                        when additional structs are needed within the resource or configuration.
///
/// By combining these attributes, developers can create powerful and flexible resources that are seamlessly integrated
/// with the Typescript environment while maintaining the safety and integrity of sensitive data.
///
#[proc_macro_attribute]
pub fn resource(attr: TokenStream, item: TokenStream) -> TokenStream {
	resource::resource(attr, item)
}
