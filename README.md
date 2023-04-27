[![GPLv3 license](https://img.shields.io/badge/license-GPLv3-blue.svg)](./LICENSE-GPL)
[![Apache 2.0 license](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](./LICENSE-APACHE)
[![Latest release](https://img.shields.io/github/v/release/nutshimit/mashin)](https://github.com/nutshimit/mashin/releases/latest)

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

## About us

Nutshimit is an innovative software company specializing in Infrastructure as Code (IaC) solutions. Our flagship product, Mashin, empowers developers to streamline infrastructure management with a secure, user-friendly, and efficient scripting engine. Built on top of Deno and Rust, Mashin combines the power of TypeScript with the flexibility and safety of a sandboxed environment. 

Join us as we revolutionize the way developers create and manage cloud infrastructure.
