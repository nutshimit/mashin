[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Docs](https://img.shields.io/docsrs/mashin_sdk)](https://docs.rs/mashin_sdk)
[![Latest crates.io](https://img.shields.io/crates/v/mashin_sdk)](https://crates.io/crates/mashin_sdk)

# mashin_sdk

The Mashin SDK is a Rust library for creating custom providers and resources that can be used with the Mashin engine. It provides a set of traits and macros that simplify the process of developing new providers and resources, making it easy for developers to extend the functionality of Mashin.

## Features
- `Provider` and `Resource` traits for implementing custom providers and resources
- `construct_provider!` and `resource` macros for simplifying provider and resource creation
- `ProviderState` for managing provider state data
- `ResourceResult` for handling serialized resource state
- `CliLogger` for easy provider and resource logging
- Utility functions for merging JSON and deserializing state fields

## Getting Started
To get started with the Mashin SDK, add it as a dependency to your Cargo.toml:

```
[dependencies]
mashin_sdk = "0.1"
```

Then, you can start building your custom provider or resource by implementing the appropriate traits and using the provided macros.

## Example
Here's a simple example of how to create a custom provider and resource using the Mashin SDK:

```rust
/// Construct the provider and register resources
mashin_sdk::construct_provider!(
    my_provider,
    config = {},
    resources = [my_resource],
);

/// Create a resource
#[mashin_sdk::resource]
pub mod my_resource {
    #[mashin::config]
    pub struct Config {}

    #[mashin::resource]
    pub struct Resource {}

    #[mashin::calls]
    impl mashin_sdk::Resource for Resource {}
}
```

## Documentation

For more information on how to use the Mashin SDK see the [documentation](https://docs.rs/mashin_sdk).

***

## About us

Nutshimit is an innovative software company specializing in Infrastructure as Code (IaC) solutions. Our flagship product, Mashin, empowers developers to streamline infrastructure management with a secure, user-friendly, and efficient scripting engine. Built on top of Deno and Rust, Mashin combines the power of TypeScript with the flexibility and safety of a sandboxed environment. 

Join us as we revolutionize the way developers create and manage cloud infrastructure.
