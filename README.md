# fluent_typed_codegen

[![crates.io](https://img.shields.io/crates/v/fluent_typed_codegen)](https://crates.io/crates/fluent_typed_codegen)
[![docs.rs](https://img.shields.io/docsrs/fluent_typed_codegen)](https://docs.rs/fluent_typed_codegen/latest/fluent_typed_codegen/)
[![CI](https://img.shields.io/github/actions/workflow/status/SDA-31/fluent_typed_codegen/ci.yml?branch=main&label=CI&logo=github)](https://github.com/SDA-31/fluent_typed_codegen/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/fluent_typed_codegen)](https://crates.io/crates/fluent_typed_codegen)
[![License](https://img.shields.io/crates/l/fluent_typed_codegen)](LICENSE)

Generate a typed Rust API from modular Fluent translation files. Language folders
determine the available locales; paths within each language become named
translation scopes, with message arguments checked at compile time.

Built on [fluent-typed](https://github.com/human-solutions/fluent-typed), which
generates the typed message accessors and provides Fluent formatting. This crate
adds module discovery and a typed translation tree around that foundation.

Use it in Rust applications and libraries through a Cargo build script, or call
the generator directly from your own build tooling.

[API documentation](https://docs.rs/fluent_typed_codegen/latest/fluent_typed_codegen/) ·
[Runnable Rust example](examples/minimal/README.md)

## Setup

Add these dependencies to your application's Cargo.toml.
The declared minimum Rust version is 1.95.

```toml
[build-dependencies]
fluent_typed_codegen = { version = "0.1.2", default-features = false, features = ["build"] }

[dependencies]
fluent_typed_codegen = { version = "0.1.2", default-features = false }
fluent-typed = { version = "0.9.0", default-features = false, features = ["langneg"] }
fluent-syntax = "0.12"

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

```rust
// build.rs
fn main() -> std::process::ExitCode {
    fluent_typed_codegen::build()
}
```

`assets/localizations/localization.toml`:

```toml
translations-directory = "translations"
source-language = "en"
default-language = "en"
```

Put matching FTL files under
`assets/localizations/translations/{en,es,ru}/`, grouped by responsibility.
`translations-directory` is optional: omit it when the locale folders sit beside
`localization.toml` (`en/`, `es/`, etc.). Its default is `"."`; a configured path is
relative to that TOML, not Cargo.toml. The legacy `languages-directory` alias is
accepted, but specifying both names is an error. No directory guessing is performed.
Languages and nested modules are discovered automatically, not enumerated in Rust.
Source-language defines keys, references and argument annotations.
`default-language` is emitted as application startup metadata (`DEFAULT_LANGUAGE`);
the application selects its initial locale. `Locale::default()` identifies the
**source** language, not the configured startup language.
Unknown fields, unsafe paths, symlinked source trees, missing
languages/modules, duplicate keys and incompatible contracts fail generation.

`build` returns a failing ExitCode and readable diagnostics instead of panicking.
`from_cargo` returns Result for custom error handling. Module mismatches identify
missing and extra paths; the generator never repairs translator files automatically.

## Features and dependency boundaries

- `build` enables discovery, validation, generation and the typed extension API.
  It is enabled by default; disable it on the application's macro-only dependency.
- With `default-features = false` and no features, the crate has **no dependencies**
  and exports only the `translations!` macro. Syn, quote, prettyplease, TOML parsing
  and upstream code generation are not compiled into this instance of the crate.

The macro still needs the consumer's `fluent-typed` and `fluent-syntax` runtime
dependencies shown above; they interpret translations, not Rust source. Keep those
two dependency names canonical when using the convenience macro. The macro crate
itself may be renamed, as the example's `l10n::translations!` demonstrates.

Use Cargo resolver 2 or 3 (edition 2024 selects 3 unless a workspace overrides it).
Build-dependencies are then resolved separately from normal dependencies. An
explicit workspace-wide all-features build may still enable the generator in
normal targets; use an isolated consumer graph when checking runtime isolation.

## Outputs and indexing

All outputs stay in Cargo OUT_DIR, respecting the selected profile and target:

- `translations.rs`: `Translations`, `Locale`, named groups and file types.
- `locale_modules.rs`: original module sources and build-validated metadata.
- `validation.rs`: private checked-loading contract, emitted from the same schema
  implementation used at build time.
- `modules/<FTL path without extension>/translations.{rs,ftl}`: upstream API/bundle.
- `inputs/<inventory hash>/`: persistent staging inputs for correct Cargo reruns.

There is no combined root translations.ftl. Generation does not edit source
resources or delete unrelated/stale output. Failed output may be incomplete:
always propagate build errors. Rust-analyzer indexes through Cargo build scripts;
`cargo check` regenerates without running the application.

Cargo tracks both the language directory (new files/locales) and each original
FTL file (edits and removals), even when those assets are excluded from packaging.
Removing a module from every language regenerates the API; removing it from only
one language reports a contract mismatch. Restoring matching files recovers on the
next build. Upstream's staged-input timestamps can require one additional rebuild
after an edit; subsequent unchanged builds reuse the generated output.

## Typed translation scopes

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn draw(translations: &texts::Translations) {
    let presentation: &texts::Presentation = translations.presentation();
    let hud: &texts::presentation::Hud = presentation.hud();
    println!("{}", hud.msg_title());
}
```

This ordinary `macro_rules!` declares the module, imports the two Fluent libraries
and includes the caller's Cargo output. It accepts module attributes, any Rust
visibility and an optional trailing semicolon inside the invocation. It does not
generate files, choose a language, register resources or install any framework.
The build script remains necessary: build-dependencies alone do not make a macro
available in application source.

Manual inclusion remains supported for custom dependency aliases or frontend
layouts; existing raw consumers do not need to add the macro dependency. Import
their runtime libraries as `fluent_typed` and `fluent_syntax` inside the generated
module, then include `concat!(env!("OUT_DIR"), "/translations.rs")`.

Folders expose snake_case modules and named group types. Each FTL file exposes
only a PascalCase leaf type, such as `presentation::Hud`, with upstream accessors
through Deref; there is no public `presentation::hud` module. Additional upstream
parameter/structured-result types are re-exported beside the leaf with its name
as a prefix, such as `presentation::HudPrompt`. Collisions with sibling scope or
message type names are reported during generation.
Clones share immutable catalogs through Arc.
Equal keys in different files remain independent and may have different types.

`Locale::load()` and `Translations::embedded(locale)` parse embedded FTL data.
`Translations::from_modules(locale, &[("presentation/hud.ftl", source), ...])`
checks complete path inventory, exact keys/variables/references and upstream typed
contracts before returning a snapshot. Prose-only edits work; missing/extra/
duplicate modules, removed variables and changed references fail. These methods
perform no filesystem reads: applications supply complete FTL strings, read
external files and decide when to replace a snapshot.
Raw consumers need both runtime dependencies above; framework bridges can
re-export them instead.

Names normalize to snake_case modules/accessors and PascalCase types. Keywords
use raw identifiers. Ambiguous names and file/directory collisions are rejected.
Local messages, terms and attributes may reference one another in the same file;
missing references and cycles fail. Cross-file references/shared-term imports
are not supported. Existing key prefixes are not rewritten.

## Numbers, plurals and RTL

The generator preserves upstream Fluent argument types and select expressions;
it does not format numbers or choose plural categories. Native numeric arguments
remain available through the accessors generated by **fluent-typed**.

For locale-formatted Decimal values, applications can independently use
[fluent_typed_decimal](https://github.com/SDA-31/fluent_typed_decimal)
([API reference](https://docs.rs/fluent_typed_decimal/)). It uses ICU4X to format
the number and select a plural category from the same rounded, visible value.
Declare both arguments as strings in the source-language FTL:

```ftl
# $value (String) - Already localized number.
# $plural (String) - ICU plural category, such as one or other.
remaining = { $plural ->
    [one] { $value } item left
   *[other] { $value } items left
    }
```

The resulting accessor still takes two ordinary string-compatible arguments.
Pass `number.text()` to `value` and `number.selector()` to `plural`, following
the generated parameter order. `LocalizedNumber` belongs to the Decimal adapter,
not this generator or upstream Fluent, and needs no codegen feature. Neither
the adapter nor ICU is added to this crate's dependencies.

With a String selector, Fluent matches literal keys such as `[one]` or `[few]`;
it does not recompute a numeric plural rule. Each translation can use its own
category branches. An unmatched string selects the starred default branch.
Numeric exact matches such as `[1]` require a native numeric selector instead.
Use separate messages for domain states such as an empty inventory when appropriate.

Keep the formatter's language aligned with the loaded translation snapshot.
Arabic digits and plural rules are distinct from right-to-left presentation:
Fluent handles interpolation isolation, while the application's text renderer
owns bidi layout, shaping and fonts. This generator does not reverse strings or
implement a rendering engine.

## Framework extensions

An optional `Extension` decorates an additional entrypoint with typed Rust syntax
for application-specific traits, attributes or registration code:

- `root_imports` / `scope_imports`: `Vec<syn::ItemUse>`.
- `type_attributes`: `Vec<syn::Attribute>`.
- `type_declaration`: a complete annotated `syn::ItemStruct` in, `syn::Item` out.
  Its default preserves the declaration. Integrations can wrap it in a
  runtime-owned item macro without moving dependency-feature decisions into the
  build script. Preserve the name, visibility, fields and existing attributes.
- `root_items`: `Vec<syn::Item>`.
- `Scope`: a `syn::Path` and a sequence of `syn::Ident` accessor names.

Scope descriptors follow deterministic preorder; extension-specific reserved names
are checked before output. Syn is re-exported so adapters can use the exact syntax
types and `parse_quote!` without managing another dependency/version themselves.
The plain tree is always generated unchanged.

Use `build_with`, `from_cargo_with` or `generate_with` for such a frontend.
Hooks construct trusted syntax, do not perform I/O and own their dependency aliases.
Output filenames must be direct .rs children and cannot overwrite core outputs.
For example, an attribute hook contains ordinary Rust-like syntax, not escaped strings:

```rust
use fluent_typed_codegen::syn::{Attribute, parse_quote};

fn type_attributes() -> Vec<Attribute> {
    vec![parse_quote!(#[allow(dead_code)])]
}
```

See the `Extension` Rustdoc for a complete compiled example. Dynamic wrappers,
locales and metadata are assembled with `quote!`; the complete file is parsed by
Syn and printed by `prettyplease` into target/ only. No formatter process is needed,
and this does not change the source repository's cargo fmt policy. Unmodified
upstream module output and the shared Fluent validator retain their own formatting.

Syntax nodes improve construction and syntax diagnostics; they do not resolve
Rust types or imports. Consumer compilation remains necessary. Syn's opaque
`Verbatim` fallback nodes are rejected recursively with a filename-tagged error
before printing; emit structured supported syntax instead. Macro token bodies
are left to Rust. The generator tests exercise syntax hooks with a standalone
extension. Integrations should also compile a consumer of their emitted API.

## Custom build tooling

`Settings::from_manifest` and `generate(package, output, settings)` support other
build frontends. Choose a tool-owned directory beneath target/.

```sh
git clone https://github.com/SDA-31/fluent_typed_codegen.git
cd fluent_typed_codegen
# Replace the input path with any package configured as shown in Setup.
cargo run --manifest-path Cargo.toml --example generate -- /path/to/consumer target/localization-example-generated
cargo test --manifest-path Cargo.toml
cargo doc --manifest-path Cargo.toml --no-deps
```

These commands work from a standalone generator checkout. Add `--locked --offline`
after the first dependency resolution. The local lockfile and target directory
are ignored; a consuming workspace owns its own lockfile.
The [bundled example](examples/minimal/README.md) uses repository-local paths for
development; external applications use the registry dependencies in Setup.

The generator owns discovery, validation and emitted Rust. Your application owns
language selection, resource loading and when to replace an existing snapshot.
Use the generated checked `Translations::from_modules` API for external catalogs;
the generator itself does not install a filesystem watcher or a UI update loop.

## Continuous integration

[CI workflow](.github/workflows/ci.yml) runs on pushes (including tags), pull
requests and manual dispatch. It checks this repository independently:

- Generator tests, doctests, macro-only dependency isolation and the generated
  consumer on Linux, Windows and macOS with stable Rust.
- The same test suite on Linux with the declared minimum Rust 1.95.0.
- Formatting, Clippy with warnings denied, Rustdoc in both feature modes and
  compilation of the packaged archive on Linux.

Nested example packages are checked explicitly; no enclosing application or
workspace lockfile is needed. CI resolves fresh standalone lockfiles, then uses
`--locked`. Dependency/build caches are optional accelerators, not prerequisites.
Unix retains quoted-path coverage while all platforms exercise spaced paths.

Actions are pinned to commit SHAs, checkout credentials are not retained, and
the workflow has only `contents: read` permission. There is no publishing job,
registry token or automatic release. Pushing a version tag to GitHub runs checks only;
publication to crates.io is a separate, deliberate manual step.

## License

[MIT](LICENSE). This license covers the generator repository, not a consuming
application or its translation assets. Third-party dependencies retain their
respective licenses.
