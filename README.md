# fluent_typed_codegen

[![crates.io](https://img.shields.io/crates/v/fluent_typed_codegen)](https://crates.io/crates/fluent_typed_codegen)
[![docs.rs](https://img.shields.io/docsrs/fluent_typed_codegen)](https://docs.rs/fluent_typed_codegen/latest/fluent_typed_codegen/)
[![CI](https://img.shields.io/github/actions/workflow/status/SDA-31/fluent_typed_codegen/ci.yml?branch=main&label=CI&logo=github)](https://github.com/SDA-31/fluent_typed_codegen/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/fluent_typed_codegen)](https://crates.io/crates/fluent_typed_codegen)
[![License](https://img.shields.io/crates/l/fluent_typed_codegen)](LICENSE)

**Unreleased API on `feat/runtime-module-loading`.** Published 0.1.4 retains the
previous embedded API. The instructions below pin a tested Git revision of the
new API. Installing `fluent_typed_codegen = "0.1.4"` will not provide it.

Generate a typed Rust API from modular Fluent translation files. Write messages
in `.ftl` files, then call them through named Rust scopes:

```rust
translations.presentation().hud().msg_greeting("Ada")
```

Built on [fluent-typed](https://github.com/human-solutions/fluent-typed), which
generates the typed message accessors and provides Fluent formatting. This crate
adds module discovery and a typed translation tree around that foundation.

Language folders determine the available locales; file paths determine the Rust
scopes. The build script checks that translations agree on their message contracts,
and Rust checks the arguments at each call site.

[Published 0.1.4 API documentation](https://docs.rs/fluent_typed_codegen/0.1.4/fluent_typed_codegen/) ·
[Loading recipes for this API](docs/loading.md) ·
[Runnable Rust example](examples/minimal/README.md) · [Changelog](CHANGELOG.md)

## Contents

- [Create your first application](#setup)
- [Configure paths and languages](#configuration)
- [Call generated messages](#typed-translation-scopes)
- [Load files, bytes or individual modules](docs/loading.md)
- [Use an archive or translation pack](#external-storage-and-translation-packs)
- [Format numbers and plurals](#numbers-plurals-and-rtl)
- [Fix common setup and loading errors](#troubleshooting)
- [Choose dependency features](#features-and-dependency-boundaries)
- [Find generated files and IDE output](#outputs-and-indexing)
- [Write a framework integration](#framework-extensions)
- [Use custom build tooling](#custom-build-tooling)
- [Run repository checks](#continuous-integration)
- [License](#license)

## Setup

Start here for a small application. This path embeds your translations explicitly
and loads one complete language, so running the executable needs no asset setup.
Once it works, the [loading recipes](docs/loading.md) show how to switch to files,
your own buffers or modules loaded on demand.

The declared minimum Rust version is 1.95.
Future releases may raise the compiler requirement; release notes will identify
the last version supporting the previous minimum.

### 1. Create an application

With Rust 1.95 or newer installed, run:

```sh
cargo new localization-demo --edition 2024
cd localization-demo
```

Replace its `Cargo.toml` with this complete file. The build dependency generates
the API; the normal dependency includes it and supplies the manifest type.

```toml
[package]
name = "localization-demo"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"

[build-dependencies]
fluent_typed_codegen = { git = "https://github.com/SDA-31/fluent_typed_codegen", rev = "5761d2843d80c5c4bbd6328d3037de61df398fdd", default-features = false, features = ["build"] }

[dependencies]
fluent_typed_codegen = { git = "https://github.com/SDA-31/fluent_typed_codegen", rev = "5761d2843d80c5c4bbd6328d3037de61df398fdd", default-features = false }
fluent-typed = { version = "0.9.0", default-features = false, features = ["langneg"] }
fluent-syntax = "0.12"

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

### 2. Run generation from build.rs

Create `build.rs` beside `Cargo.toml`:

```rust
fn main() -> std::process::ExitCode {
    fluent_typed_codegen::build()
}
```

Generation is an explicit build step. The `translations!` macro used below only
includes its output. `build()` reports readable diagnostics and returns a failing
exit code on errors; return it from `main` so Cargo stops the build.

### 3. Add the catalogs

Create these directories and files inside `localization-demo`:

```text
assets/localizations/
├── localization.toml
└── translations/
    ├── en/presentation/hud.ftl
    └── es/presentation/hud.ftl
```

`assets/localizations/localization.toml`:

```toml
translations-directory = "translations"
source-language = "en"
default-language = "en"
```

`assets/localizations/translations/en/presentation/hud.ftl`:

```ftl
title = Dashboard
# $name (String) - Person to greet.
greeting = Hello, { $name }!
```

`assets/localizations/translations/es/presentation/hud.ftl`:

```ftl
title = Panel
greeting = ¡Hola, { $name }!
```

The source language supplies the argument annotations. Add languages and nested
files using the same layout; no Rust list of languages or modules is needed.

### 4. Use the generated API

In `src/main.rs`:

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), texts::LoadError> {
    let manifest = texts::embed_manifest!();
    let translations = texts::Translations::from_manifest(texts::Locale::En, &manifest)?;
    println!("{}", translations.presentation().hud().msg_greeting("Ada"));
    Ok(())
}
```

Run from the application directory:

```sh
cargo run
```

It prints `Hello, Ada!` (Fluent also adds invisible isolation marks around the
argument). Load `texts::Locale::Es` to use Spanish.
`cargo check` also runs generation, so rust-analyzer can index the API without
running the application. This version explicitly embeds the translation files,
so the executable needs no external files. To load files or only one module at a
time, replace `src/main.rs` with a [loading recipe](docs/loading.md).

For local API documentation matching the pinned revision, run
`cargo doc --open`. The docs.rs links describe the published release.
For localized numbers and more generated types, see
[examples/minimal](examples/minimal/README.md).

## Configuration

Paths and language policy live in two small TOML sections:

| Setting | Location | Meaning |
| --- | --- | --- |
| `asset-root` | Cargo package metadata | Directory relative to the consuming package. |
| `catalog` | Cargo package metadata | Configuration file relative to `asset-root`. |
| `translations-directory` | Catalog TOML | Locale folders relative to this TOML; defaults to `"."`. |
| `source-language` | Catalog TOML | Language defining message keys, references and argument annotations. |
| `default-language` | Catalog TOML | Startup metadata emitted as `DEFAULT_LANGUAGE`. |

Omit `translations-directory` when `en/`, `es/` and the other locale folders sit
beside the catalog TOML. The legacy `languages-directory` alias is accepted;
specifying both names is an error. The generator does not guess directory names.

The application selects its initial locale. `default-language` records that policy;
`Locale::default()` identifies the **source** language.

Unknown fields, unsafe paths, symlinked source trees, missing languages/modules,
duplicate keys and incompatible contracts fail generation. Module diagnostics
list missing and extra paths; translator files are never repaired automatically.
Use `fluent_typed_codegen::from_cargo()` for custom `Result`-based build-error handling.

## Typed translation scopes

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn draw(translations: &texts::Translations) {
    let presentation: &texts::Presentation = translations.presentation();
    let hud: &texts::presentation::Hud = presentation.hud();
    println!("{}", hud.msg_title());
}
```

Folders become snake_case modules and named group types; each FTL file becomes
a PascalCase leaf type. For the setup above:

| Catalog path | Generated type | Accessor |
| --- | --- | --- |
| Whole language | `texts::Translations` | `Translations::from_modules(locale, &modules)?` |
| `presentation/` | `texts::Presentation` | `translations.presentation()` |
| `presentation/hud.ftl` | `texts::presentation::Hud` | `translations.presentation().hud()` |

Leaf types expose upstream message accessors through `Deref`; there is no public
`presentation::hud` module. Additional parameter and structured-result types
appear beside the leaf with its name as a prefix, such as `presentation::HudPrompt`.
Equal keys in different files remain independent and may have different types.
Clones share immutable catalogs through `Arc`.

### Loading and replacing translations

The generated API accepts readable bytes without prescribing storage. These are
the available operations; [complete programs](docs/loading.md) show the inputs:

```rust
let hud = texts::presentation::Hud::new(locale, &bytes)?;
let unchecked = texts::presentation::Hud::new_unchecked(locale, &bytes)?;
texts::presentation::Hud::validate(&bytes)?;
let path = texts::presentation::Hud::PATH;
let locale = hud.locale();
```

Checked loading validates exact keys/references and upstream typed contracts.
`new_unchecked` is a **safe** method that skips those compatibility checks; UTF-8
and Fluent parsing still run. Incompatible input may later cause accessor panics
or formatting failures. No constructor retains a borrow of the supplied bytes.
Standalone validation uses temporary allocations; validating then constructing
parses again. Normal checked loading requires no separate validation call.

For complete languages, use `Translations::from_modules(locale, modules)` or
`from_modules_unchecked`. Pairs contain `(logical_path, FTL_text)`. Both reject
missing, duplicate and unknown module paths. `validate_modules(modules)` checks
without retaining a catalog. `load_all` / `load_all_unchecked` accept
`&[(Locale, &str, &str)]` and return `HashMap<Locale, Translations>`; every compiled
locale must have a complete module set.

Errors use the generated `LoadError` enum. Module diagnostics retain logical path
and, for construction, locale. Applications own fallback, caching, publication,
and module lifetime; formatting/accessors never start I/O.

### Explicit file and embedded sources

`LocalizationManifest` is one immutable source contract shared across generated
catalogs. It contains configuration and an origin or static byte references,
never parsed Fluent catalogs or a loaded-state cache. Enable the optional
`manifest` feature on the normal dependency for TOML parsing:

```rust
let manifest = fluent_typed_codegen::LocalizationManifest::from_file(runtime_path)?;
let hud = texts::presentation::Hud::from_manifest(locale, &manifest)?;
let all = texts::Translations::from_manifest(locale, &manifest)?;
```

The [file-loading recipe](docs/loading.md#read-files-through-a-manifest) includes
the exact dependency change and a complete `main`.

`from_file` reads only TOML. The leaf constructor reads only its own module for
one language; the root constructor reads the complete language. `parse(source,
origin)` accepts already obtained TOML without I/O. `config`, `file_path` and
`embedded_modules` allow engines to inspect the contract and use their own loader.
`read(locale, path)` and `read_modules(locale, paths)` expose readable bytes for
inspection without parsing. Runtime paths need not match build-time paths.

Explicit embedding uses a build-prepared recipe; the schema-generation step stays
in `build.rs`:

```rust
let complete = texts::embed_manifest!();
let hud_only = texts::embed_manifest!(module = "presentation/hud.ftl");
let hud = texts::presentation::Hud::from_manifest(locale, &hud_only)?;
```

No expanded call means no FTL payload is included by this path. A macro in
`if false` still expands. The no-argument form includes **all raw modules and all
languages**; the `module` selector includes just that logical leaf in every
language. Runtime construction decides what gets parsed, not what enters the
binary. Static embedded bytes outlive dropped parsed catalogs. The generated
macro is crate-local, including inside a `pub mod texts`; a library can expose
its own function that explicitly invokes it. Group selectors are not provided.

Migration from published 0.1.4 removes `Locale::load()` and
`Translations::embedded()`. Use an explicit manifest or prepared bytes/pairs.
`MODULES` loses its embedded-source field and `from_modules` now returns typed
`LoadError` rather than `String`. Existing typed message calls and full-tree
getters retain their shape. There is no decompressor API.

### Naming and references

Names normalize to snake_case modules/accessors and PascalCase types. Keywords
use raw identifiers. Ambiguous names and file/directory collisions are rejected.
Local messages, terms and attributes may reference one another in the same file;
missing references and cycles fail. Cross-file references/shared-term imports
are not supported. Existing key prefixes are not rewritten.

## External storage and translation packs

Generated `Translations::from_modules` accepts UTF-8 FTL strings from any storage:
loose files, decoded archive entries, a cache or application-owned buffers. It
does not open files or implement archive formats. For example, after a caller has
collected one complete language in a `BTreeMap<String, String>` named `files`:

```rust
let modules: Vec<_> = files.iter()
    .map(|(path, source)| (path.as_str(), source.as_str()))
    .collect();
let next = texts::Translations::from_modules(locale, &modules)?;
current = next; // Replace only after the complete candidate validates.
```

Keys are paths **below the locale directory**, e.g. `ui/menu.ftl`, not
`en/ui/menu.ftl`, absolute paths or `pack://...` URLs. Input order does not matter;
the returned snapshot owns its parsed data, so callers can release input buffers.
Supply one consistent revision: schema checks cannot detect mixed but individually
compatible prose revisions. The application owns publication and UI updates.

Build-time discovery still reads the source tree through an explicit `build.rs`.
There is no archive option in the generator. For runtime distribution, package
the original per-language FTL files, not generated Rust or intermediate bundles
under `OUT_DIR`. `MODULES` records `(locale, relative module path)` without translation data;
`CATALOG_ASSET_PATH` and `LANGUAGES_DIRECTORY` describe the definition and layout.
`ASSET_ROOT` is a build-time location, not a runtime storage requirement.

Bevy consumers additionally package the original definition TOML and preserve
its relative directory layout. The
[Bevy source integration](https://github.com/SDA-31/bevy_fluent_typed/blob/feat/runtime-module-loading/docs/asset-sources.md)
uses this same checked parser through a named asset source, then updates resources
and bound text. Other applications can call `from_modules` directly.

Compatible prose updates need no new executable. Compiled locales, module paths
and typed contracts still require regeneration when changed. There is no implicit
embedded fallback; applications explicitly choose any fallback source.

## Numbers, plurals and RTL

The generator preserves upstream Fluent argument types and select expressions;
it does not format numbers or choose plural categories. Native numeric arguments
remain available through the accessors generated by **fluent-typed**.

For localized numbers, percentages, currencies and dates, it's recommended to use
a dedicated library, such as [ICU](https://unicode-org.github.io/icu/userguide/format_parse/)
or its Rust-oriented [ICU4X components](https://docs.rs/icu/). Their APIs and
available features differ; check the selected component's stability and input
semantics. Formatting is application work, not a generator feature.

For exact decimal text, use
[ICU4X DecimalFormatter](https://docs.rs/icu_decimal/latest/icu_decimal/struct.DecimalFormatter.html).
When grammar must follow the same visible precision, apply rounding once, format
that Decimal and pass it to
[ICU4X PluralRules](https://docs.rs/icu_plurals/latest/icu_plurals/struct.PluralRules.html).
Map the resulting category to a CLDR keyword. Declare both arguments as strings
in the source-language FTL:

```ftl
# $value (String) - Already localized number.
# $plural (String) - ICU plural category, such as one or other.
remaining = { $plural ->
    [one] { $value } item left
   *[other] { $value } items left
    }
```

Call `msg_remaining(selector, text)` with the category keyword and formatter's
string, in that generated order. ICU belongs in the application's dependencies;
no generator feature or generated numeric type is needed. Validate ICU's
documented operand limits for unconstrained input precision.

### ICU example

The [runnable example](examples/minimal/src/main.rs) combines number formatting
and plural selection. Its ICU dependencies belong to the application:

```toml
icu_decimal = { version = "2.3", features = ["alloc"] }
icu_locale_core = "2.3"
icu_plurals = "2.3"
```

Create the catalog and reusable formatters once for the selected locale:

```rust
let translations = texts::Translations::from_manifest(texts::Locale::Ru, &texts::embed_manifest!())?;
let language: icu_locale_core::Locale = translations.locale().as_ref().parse()?;
let formatter = icu_decimal::DecimalFormatter::try_new((&language).into(), Default::default())?;
let rules = icu_plurals::PluralRules::try_new_cardinal((&language).into())?;
```

Then format a number and pass it to the generated message:

```rust
let amount = icu_decimal::input::Decimal::from(22);
let text = formatter.format_to_string(&amount);
let selector = plural_key(rules.category_for(&amount));
let label = translations.numbers().msg_remaining(selector, text);
```

For the example's Russian translation, `label` contains `Осталось 22 предмета`
(with Fluent's isolation marks around the number).
[`plural_key`](examples/minimal/src/main.rs) is the example's small enum-to-keyword
match (`One` → `"one"`, `Few` → `"few"`, etc.), not a generator API or another
plural-rule implementation. The [tests](examples/minimal/tests/unit/catalog.rs) also cover
`1` versus visible `1.0` and native Fluent selectors.

The catalog and formatters can be kept in application state and reused across
messages. When the language changes, select the matching catalog and formatters
together and recompute the strings. Changes to values or formatting settings
also require fresh text. The application decides when to recompute strings and
how to use them.

### Selector and rendering boundaries

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

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| `Hud::new`, `from_manifest` or `embed_manifest!` is missing | Use the Git dependencies from [Setup](#setup) in **both** Cargo sections. Published 0.1.4 uses the previous API. |
| `translations!` cannot find `OUT_DIR` or `translations.rs` | Add the [build.rs](#2-run-generation-from-buildrs) beside the application's Cargo.toml and enable `build` on its build dependency. Resolve any earlier generation error first. |
| `LocalizationManifest::from_file` or `parse` is missing | Enable `manifest` on the **normal** dependency, as in the [file recipe](docs/loading.md#read-files-through-a-manifest). A build dependency's features do not enable runtime APIs. |
| Generated code cannot resolve `fluent_typed` or `fluent_syntax` | Keep `fluent-typed` and `fluent-syntax` under those canonical dependency names in the application's `[dependencies]`. |
| Runtime file loading reports a missing file | Run the recipe from the application directory. In a packaged application, pass the installed TOML path; retain its relative translation layout. Cargo's `asset-root` does not set the runtime working directory. |
| Loading reports missing, duplicate or unexpected modules | Pass each logical path once, without a locale prefix. `from_modules` needs a complete language; use `Hud::new` for just one leaf. |
| `load_all` reports missing languages | Supply every compiled locale. To keep only one language, use `from_modules` or `from_manifest`. |
| `Locale::default()` does not match `default-language` | `Locale::default()` is the source language. Choose the startup locale explicitly; `DEFAULT_LANGUAGE` is emitted metadata. |
| An accessor fails after unchecked loading | Use the checked constructor to diagnose the mismatch. Skipping contract checks does not make incompatible messages valid. |

Changes to language folders, module paths or typed message contracts require a
rebuild. Compatible edits to external FTL can be loaded again without rebuilding.
There is no automatic watcher, fallback or cross-file term import in this crate.
Your application decides when to replace or release each loaded module.

## Features and dependency boundaries

| Feature selection | API and dependencies |
| --- | --- |
| `build` (default) | Discovery, validation, generation and the typed extension API. Use in build-dependencies. |
| `default-features = false`, no features | `translations!` plus std-only manifest contracts; zero external dependencies. |
| `manifest` | Runtime TOML parsing and `LocalizationManifest::from_file` / `parse`; no generator or Fluent dependencies. |

The macro declares a module, imports the consumer's `fluent-typed` and
`fluent-syntax` libraries, and includes Cargo output. Keep those two dependency
names canonical. The macro crate itself can be renamed, as the example's
`l10n::translations!` demonstrates. It accepts module attributes, any Rust visibility
and an optional trailing semicolon inside the invocation.

Use Cargo resolver 2 or 3 (edition 2024 selects 3 unless a workspace overrides it)
to separate build and normal feature contexts. Syn, quote, prettyplease, TOML
parsing and upstream code generation stay out of the macro-only instance. A
workspace-wide all-features build may enable them in normal targets; verify
runtime isolation with an isolated consumer graph.

Manual inclusion supports custom runtime dependency aliases and frontend layouts.
Import the runtime libraries as `fluent_typed` and `fluent_syntax`, and this crate
as `__fluent_codegen`, inside the generated module, then include
`concat!(env!("OUT_DIR"), "/translations.rs")`. The std-only support dependency
is required even for manual inclusion. Framework bridges may re-export it and
the Fluent runtime libraries under those aliases.

## Outputs and indexing

All Cargo outputs stay in `OUT_DIR`, respecting the selected profile and target:

| Output | Purpose |
| --- | --- |
| `translations.rs` | `Translations`, `Locale`, groups and file types. |
| `locale_modules.rs` | Build-validated locale/path metadata, with no FTL payload. |
| `validation.rs` | Private checked-loading contract shared with build-time validation. |
| `modules/<FTL path without extension>/translations.{rs,ftl}` | Upstream API and bundle for each file. |
| `inputs/<inventory hash>/` | Persistent staging inputs for Cargo reruns. |

There is no combined root `translations.ftl`. Generation does not edit source
resources or delete unrelated/stale output. Failed generation may leave partial
output, so always propagate build errors.

Cargo tracks the locale directory and every original FTL file, including sources
excluded from packaging. Adding or removing modules in every language regenerates
the API; changing only one language reports a mismatch. Restoring matching files
recovers on the next build. Upstream's staged-input timestamps can require one
additional rebuild after an edit; subsequent unchanged builds reuse the output.

## Framework extensions

Implement
[`Extension`](src/extension.rs)
to generate an additional entrypoint with framework traits, attributes or
registration code. The plain tree is always generated unchanged.

| Hook or descriptor | Syntax types |
| --- | --- |
| `root_imports`, `scope_imports` | `Vec<syn::ItemUse>` |
| `type_attributes` | `Vec<syn::Attribute>` |
| `type_declaration` | Annotated `syn::ItemStruct` in, `syn::Item` out |
| `root_items` | `Vec<syn::Item>` |
| `Scope` | `type_path`, typed `accessors`, `logical_path`, and optional leaf `module_path` |

Use `build_with`, `from_cargo_with` or `generate_with` to supply the extension.
The re-exported `syn` provides matching syntax types and `parse_quote!`:

```rust
use fluent_typed_codegen::syn::{Attribute, parse_quote};

fn type_attributes() -> Vec<Attribute> {
    vec![parse_quote!(#[allow(dead_code)])]
}
```

See the linked trait documentation for a complete compiled example. Scope
descriptors follow deterministic preorder; reserved names are checked before output.
Hooks construct trusted syntax, perform no I/O and own their dependency aliases.
Output filenames must be direct `.rs` children and cannot overwrite core outputs.

Generated groups expose a hidden `__from_parts(locale, children...)` integration
hook. It accepts already loaded immediate children, checks their locales and
assembles a complete scope without copying source buffers or reparsing Fluent.

The default `type_declaration` preserves the struct. A wrapper must preserve its
name, visibility, fields and attributes; a runtime-owned item macro can handle
runtime dependency features without moving those decisions into the build script.

Use `quote!` for dynamic syntax. Generated files are parsed by Syn and printed by
`prettyplease` into the output directory; no formatter process changes handwritten
sources. Opaque `Verbatim` nodes are rejected recursively before printing, while
macro token bodies are left to Rust. Syntax validation does not resolve types or
imports: compile a consumer of each extension's emitted API.

## Custom build tooling

`Settings::from_manifest` and `generate(package, output, settings)` support other
build frontends. Choose a tool-owned directory beneath target/.

```sh
git clone --branch feat/runtime-module-loading https://github.com/SDA-31/fluent_typed_codegen.git
cd fluent_typed_codegen
# Replace the input path with a package configured as shown in Setup.
cargo run --manifest-path Cargo.toml --example generate -- /path/to/consumer target/localization-example-generated
cargo test --manifest-path Cargo.toml
cargo doc --manifest-path Cargo.toml --no-deps
```

These commands work from a standalone generator checkout. Add `--locked --offline`
after the first dependency resolution. The local lockfile and target directory
are ignored; a consuming workspace owns its own lockfile.
The [bundled example](examples/minimal/README.md) uses repository-local paths for
development; external applications use the pinned Git dependencies in Setup.

## Continuous integration

[CI workflow](.github/workflows/ci.yml) runs on pushes (including tags), pull
requests and manual dispatch. It checks this repository independently:

- Generator/manifest tests, doctests, macro-only dependency isolation, optional
  parser-only builds, opt-in embedding compilation and the generated consumer on Linux, Windows and macOS with stable Rust.
- The same test suite on Linux with the declared minimum Rust 1.95.0.
- Formatting, Clippy with warnings denied, Rustdoc in both feature modes and
  compilation of the packaged archive on Linux.

CI checks nested examples explicitly and resolves fresh standalone lockfiles
before using `--locked`; no enclosing application is needed. All platforms exercise
spaced paths, and Unix also covers quoted paths.

Actions are pinned to commit SHAs with `contents: read` permission and no retained
checkout credentials. Tags run checks only; crates.io publication is manual.

## License

[MIT](LICENSE). This license covers the generator repository, not a consuming
application or its translation assets. Third-party dependencies retain their
respective licenses.
