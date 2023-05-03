[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Latest release](https://img.shields.io/github/v/release/nutshimit/mashin?label=mashin)](https://github.com/nutshimit/mashin/releases/latest)
[![Latest crates.io](https://img.shields.io/crates/v/mashin_sdk?label=sdk)](https://crates.io/crates/mashin_sdk)
[![post-release](https://github.com/nutshimit/mashin/actions/workflows/post-release.yml/badge.svg)](https://github.com/nutshimit/mashin/actions/workflows/post-release.yml)
# Mashin

Mashin is an infrastructure-as-code (IaC) engine that focuses on managing resources and providers. It enables users to define, provision, and manage infrastructure components in a reliable and efficient manner. Mashin is designed with simplicity and extensibility in mind, allowing developers to create custom providers and resources that can be utilized by operations teams to deploy infrastructure.

## Features

- Secure and isolated V8 JavaScript runtime
- TypeScript support out of the box (built on top of [deno_core](https://crates.io/crates/deno_core))
- Fully sandboxed execution environment
- Easy-to-use provider system with Rust-powered plugins

[and much more](https://github.com/nutshimit/mashin_paper/wiki/1.-Introduction)...

## Install

Shell (Mac, Linux):

```bash
curl -fsSL https://get.mashin.land | sh
```

or [download latest release](https://github.com/nutshimit/mashin/releases/latest).

## Getting started

Mashin is a command-line tool that can be used to execute (remote) scripts and manage infrastructure.

Try [running a simple provider](https://github.com/nutshimit/mashin_provider_starter):
```
mashin run https://raw.githubusercontent.com/nutshimit/mashin_provider_starter/dev/examples/my_provider.ts
```

## Build from source

To build Mashin from source, you will first need to install Rust and Cargo. 
Then, clone the repository and build the project using the following commands:

```bash
cargo build --release
```

## Contributing

Mashin is a thriving open-source project that seeks to empower developers and DevOps professionals by providing a robust infrastructure as code (IaC) solution. We believe that the best way to create a truly exceptional tool is by fostering an inclusive, collaborative community.

We welcome contributors from all backgrounds, regardless of experience, to help us improve and expand Mashin. Whether you're a Rust developer, a DevOps expert, or simply someone with innovative ideas and suggestions, your contributions are highly valued.

To contribute, please read our [contributing instructions](https://docs.mashin.land/docs/engage).
