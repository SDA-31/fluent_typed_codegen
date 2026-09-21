//! Generate typed Rust translation APIs from modular Fluent catalogs.
//!
//! Language directories are discovered automatically. FTL file paths become
//! named Rust types, and message parameters become typed accessor arguments.
//!
//! Built on [fluent-typed](https://docs.rs/fluent-typed/0.9.0/fluent_typed/), which
//! generates the typed message accessors and provides Fluent formatting. This crate
//! adds module discovery and a typed translation tree around that foundation.
//! Applications choose their own resource loading, active language and presentation layer.
//!
//! # Feature selection
//!
//! - **`build` (default):** discovery, schema validation, source generation and
//!   the `Extension` API for custom integrations. Enable in build-dependencies.
//! - **No default features:** only the dependency-free [`translations!`] macro.
//!   Use this configuration in normal dependencies. The generated code still
//!   requires the application's `fluent-typed` and `fluent-syntax` dependencies.
//!
//! Cargo resolver 2/3 keeps the build and normal feature contexts separate.
//! See the [dependency setup](https://github.com/SDA-31/fluent_typed_codegen#setup)
//! for the build and runtime dependency declarations.
//!
//! # Configure the consuming package
//!
//! ```toml
//! [package.metadata.localization]
//! asset-root = "assets"
//! catalog = "localizations/localization.toml"
//! ```
//!
//! The asset root is relative to the Cargo package. The configuration path is
//! relative to that root. Its filename is configurable; the TOML contains:
//!
//! ```toml
//! translations-directory = "translations"
//! source-language = "en"
//! default-language = "en"
//! ```
//!
//! `translations-directory` is optional and relative to this TOML. Omit it to use
//! locale folders beside the file (default `.`). The legacy `languages-directory`
//! alias is accepted, but specifying both names is an error.
//!
//! With the explicit path above, put matching FTL modules under
//! `assets/localizations/translations/en/`, `es/`
//! and any other locale directories. The source language defines message keys,
//! references and argument annotations. `default-language` is emitted as
//! `DEFAULT_LANGUAGE` metadata for application startup; `Locale::default()`
//! identifies the source language. Each language must provide the same contract.
//!
//! # Generate and use the API
//!
//! Return `fluent_typed_codegen::build()` from the consuming `build.rs` to run
//! validation and generation. Errors produce readable diagnostics and a failing
//! exit code. `cargo check` also regenerates for IDE indexing; no application
//! execution is needed. Source files are never rewritten.
//!
//! For a file `ui/menu.ftl` containing `title = Settings`, the generated type is
//! `texts::ui::Menu`. A directory becomes a namespace/group, a file becomes a
//! leaf type, and each Fluent message has a typed accessor:
//!
//! ```ignore
//! fluent_typed_codegen::translations!(pub mod texts);
//!
//! let translations: texts::Translations = texts::Locale::En.load();
//! let ui: &texts::Ui = translations.ui();
//! let menu: &texts::ui::Menu = ui.menu();
//! println!("{}", menu.msg_title());
//! ```
//!
//! This snippet needs consumer-owned FTL and build output; the
//! [runnable Rust example](https://github.com/SDA-31/fluent_typed_codegen/tree/main/examples/minimal)
//! demonstrates the complete setup. There is no public module at a leaf path.
//! Message keys in different files stay independent, including their argument types.
//!
//! # Output and checked loading
//!
//! Cargo entrypoints emit Rust, per-module FTL and source metadata into `OUT_DIR`.
//! Keep that output under target/ and out of source control. `generate` also
//! accepts an explicit tool-owned output directory for custom build frontends.
//! Failed generation can leave partial output; always propagate build errors.
//!
//! `Locale::load()` parses embedded FTL. Generated `Translations::from_modules`
//! validates and parses caller-supplied FTL strings before
//! returning a snapshot. Compatible prose edits can be loaded without rebuilding;
//! changes to the schema or language/module inventory require regeneration.
//! These runtime methods perform no filesystem reads. File loading and replacing
//! snapshots are caller-controlled, not a filesystem watcher.
//!
//! # Numbers, plural selection and presentation
//!
//! Native Fluent numeric selectors remain available; this generator does not
//! replace upstream argument types or select-expression behavior. For numbers,
//! percentages, currencies and dates, it's recommended to use dedicated
//! [ICU](https://unicode-org.github.io/icu/userguide/format_parse/) or
//! [ICU4X](https://docs.rs/icu/) formatters. Formatting policies and component
//! stability belong to the application.
//! For decimal text, use [DecimalFormatter](https://docs.rs/icu_decimal/latest/icu_decimal/struct.DecimalFormatter.html).
//! If grammar must follow visible precision, pass the same prepared Decimal to
//! [PluralRules](https://docs.rs/icu_plurals/latest/icu_plurals/struct.PluralRules.html).
//! Annotate the displayed text and category keyword as `(String)` in source FTL.
//! The [runnable example](https://github.com/SDA-31/fluent_typed_codegen/tree/main/examples/minimal)
//! uses ICU directly; no special generated numeric type, generator dependency or
//! feature is required. Respect ICU's operand limits for arbitrary-precision input.
//! A [compact ICU walkthrough](https://github.com/SDA-31/fluent_typed_codegen#icu-example)
//! shows reusable formatters and a numeric input. Catalogs and formatters can be
//! kept in application state; the application decides when to recompute strings
//! and how to use the result.
//!
//! String categories match literal Fluent keys such as `[one]`; unmatched strings
//! select the starred default. Numeric exact matches such as `[1]` still require
//! a numeric selector. Keep number formatting aligned with the snapshot's locale.
//! Number formatting and Fluent interpolation isolation do not implement RTL
//! layout, glyph shaping or fonts; those belong to the application's renderer.
//!
//! # Custom integrations
//!
//! With `build`, `Extension` adds an entrypoint decorated with typed Rust syntax,
//! while preserving the plain generated API. It can add attributes, imports or
//! application-specific registration code. See the
//! [extension guide](https://github.com/SDA-31/fluent_typed_codegen#framework-extensions)
//! for its syntax hooks and consumer-compilation requirements.
//! The repository links follow `main`; this API reference describes the viewed version.
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
