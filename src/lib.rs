//! Generate typed Rust translation APIs from modular Fluent catalogs.
//!
//! See the
//! [setup guide](https://github.com/SDA-31/fluent_typed_codegen/tree/main#setup)
//! for installation, and the
//! [migration guide](https://github.com/SDA-31/fluent_typed_codegen/blob/main/docs/migration-0.2.md)
//! when upgrading from 0.1.4. For 0.2.1 applications, see the
//! [0.2.2 migration](https://github.com/SDA-31/fluent_typed_codegen/blob/main/docs/migration-0.2.2.md).
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
//! - **`manifest`:** optional TOML parsing and file-manifest factory.
//! - **No default features:** [`translations!`] and std-only source contracts, with no dependencies.
//!   Use this configuration in normal dependencies. The generated code still
//!   requires the application's `fluent-typed` and `fluent-syntax` dependencies.
//!
//! Cargo resolver 2/3 keeps the build and normal feature contexts separate.
//! See the [dependency setup](https://github.com/SDA-31/fluent_typed_codegen/tree/main#setup)
//! for the build and runtime dependency declarations.
//!
//! # Configure the consuming package
//!
//! ```toml
//! [package.metadata.localization]
//! catalog = "assets/localizations/localization.toml"
//! ```
//!
//! `catalog` is a filesystem path relative to the package's Cargo.toml; `..` can
//! locate shared sources. Its filename is configurable. The generator does not
//! select an engine asset root. The TOML contains:
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
//! let manifest = texts::embed_manifest!();
//! let translations = texts::Translations::from_manifest(texts::Locale::En, &manifest)?;
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
//! Generated leaves expose checked `new(locale, bytes)`, safe `new_unchecked`,
//! and standalone `validate`. `Translations::from_modules` validates and parses
//! caller-supplied FTL strings before
//! returning a snapshot. Compatible prose edits can be loaded without rebuilding;
//! changes to the schema or language/module inventory require regeneration.
//! These runtime methods perform no filesystem reads. File loading and replacing
//! snapshots are caller-controlled, not a filesystem watcher.
//! Sources can come from decoded archive entries or any application-owned buffers;
//! module keys remain paths below the locale directory, not storage URLs. The
//! returned snapshot retains no input borrows. Read a coherent revision before
//! parsing and replace application state only after validation succeeds. Archive
//! I/O and pack installation remain application choices; there is no implicit fallback.
//! Generated `from_manifest` constructors read files through a
//! [`LocalizationManifest`] contract, while
//! `texts::embed_manifest!()` explicitly includes a build-prepared raw source set.
//! Without that macro invocation, generated APIs contain no FTL payload.
//! The embedded macro is crate-local. It accepts only an empty invocation for the
//! complete tree or a block of named manifest constants for selected scopes.
//! Ordinary catalog imports and aliases remain unrestricted.
//! Unselected FTL is not included even in unoptimized builds without LTO or stripping.
//!
//! Named embedded manifests use selectors relative to this translation tree:
//!
//! ```ignore
//! texts::embed_manifest! {
//!     pub const MENU = ui::Menu;
//!     const COMPLETE = Translations;
//! }
//!
//! use texts::ui::Menu as Interface;
//! let menu: Interface = Interface::from_manifest(texts::Locale::En, &MENU)?;
//! ```
//!
//! These constants have type [`LocalizationManifest`]. They contain source bytes,
//! not parsed catalogs. A private localization module can export its constants and
//! catalog aliases without exposing its generated tree. Selectors are generated
//! schema paths, independent of application imports; strings and type aliases are
//! rejected as selectors. Attributes such as `#[cfg(...)]` apply to each declaration.
//! Reading bytes borrows the selected static data; [`LocalizationManifest::config`]
//! initializes shared metadata on first access. Bind `let manifest = &MENU;` before
//! retaining a borrow of that metadata. Fluent parsing happens in `from_manifest`.
//!
//! See the [complete loading recipes](https://github.com/SDA-31/fluent_typed_codegen/blob/main/docs/loading.md)
//! for bytes, files, embedding, all languages and explicit module lifetimes.
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
//! A [compact ICU walkthrough](https://github.com/SDA-31/fluent_typed_codegen/tree/main#icu-example)
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
//! [extension guide](https://github.com/SDA-31/fluent_typed_codegen/tree/main#framework-extensions)
//! for its syntax hooks and consumer-compilation requirements.
//! Repository links follow main; this API reference describes the viewed version.
#![cfg_attr(feature = "build", doc = include_str!("../docs/errors.md"))]
#![warn(missing_docs)]
mod macros;
mod manifest;

#[cfg(test)]
mod manifest_tests;

pub use manifest::{LocalizationManifest, ManifestError};

#[cfg(feature = "build")]
mod build_error;
#[cfg(feature = "build")]
mod build_io;
#[cfg(feature = "build")]
pub use build_error::{
	BuildError, IoOperation, ModuleMismatch, NameError, NameOwner, SyntaxNode, UpstreamShapeError,
};
#[cfg(feature = "build")]
pub use schema::SchemaError;
mod config_error;
mod configuration;
pub use config_error::{ConfigError, ConfigField, FieldError, PathError};
#[cfg(feature = "build")]
mod diagnostics;
#[cfg(feature = "build")]
mod discovery;
#[cfg(feature = "build")]
mod embedding;
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
