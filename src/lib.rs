//! Bevy-free discovery and generation of typed Fluent catalogs.
//!
//! Add this crate to build-dependencies with the `build` feature and return
//! `build()` from build.rs. `Settings` locates resources and localization.toml through Cargo
//! metadata; this generator validates configuration and Fluent module schemas.
//! All languages and nested modules are discovered, with no fixed language list.
//!
//! Cargo entrypoints write generated Rust, FTL and runtime metadata to OUT_DIR.
//! `generate` supports an explicit, caller-owned output directory for custom
//! frontends; keep it under target/ rather than in source resources.
//! Framework adapters use the optional `Extension` syntax hooks. This crate
//! neither knows their filenames nor depends on a game engine or runtime adapter.
//!
//! [`translations!`] is available without features or dependencies. Add this crate
//! to normal dependencies with `default-features = false` to declare the generated
//! module. Its Fluent runtime dependencies remain consumer-owned. `build` is on
//! by default for existing build-script consumers; Cargo resolver 2/3 keeps host
//! generation separate from the dependency-free macro used by the application.
#![warn(missing_docs)]
mod macros;

#[cfg(feature = "build")]
mod configuration;
#[cfg(feature = "build")]
mod diagnostics;
#[cfg(feature = "build")]
mod discovery;
#[cfg(feature = "build")]
mod extension;
#[cfg(feature = "build")]
mod generation;
#[cfg(feature = "build")]
mod locale;
#[cfg(feature = "build")]
mod message_types;
#[cfg(feature = "build")]
mod metadata;
#[cfg(feature = "build")]
mod paths;
#[cfg(feature = "build")]
mod references;
#[cfg(feature = "build")]
mod render;
#[cfg(feature = "build")]
mod schema;
#[cfg(feature = "build")]
mod settings;
#[cfg(feature = "build")]
mod source;
#[cfg(feature = "build")]
mod tree;
#[cfg(feature = "build")]
mod upstream;

#[cfg(feature = "build")]
pub use configuration::CatalogConfig;
#[cfg(feature = "build")]
pub use extension::{Extension, Scope};
#[cfg(feature = "build")]
pub use generation::{build, build_with, from_cargo, from_cargo_with, generate, generate_with};
#[cfg(feature = "build")]
pub use settings::Settings;

/// Rust syntax types and `parse_quote!` used by the extension contract.
/// Re-exported so adapters do not need to choose a matching Syn major version.
#[cfg(feature = "build")]
pub use syn;

#[cfg(all(test, feature = "build"))]
mod contracts_tests;

#[cfg(all(test, feature = "build"))]
mod tests;
