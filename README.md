# fluent_typed_codegen

[![crates.io](https://img.shields.io/crates/v/fluent_typed_codegen)](https://crates.io/crates/fluent_typed_codegen)
[![docs.rs](https://img.shields.io/docsrs/fluent_typed_codegen)](https://docs.rs/fluent_typed_codegen/latest/fluent_typed_codegen/)
[![CI](https://img.shields.io/github/actions/workflow/status/SDA-31/fluent_typed_codegen/ci.yml?branch=main&label=CI&logo=github)](https://github.com/SDA-31/fluent_typed_codegen/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/fluent_typed_codegen)](https://crates.io/crates/fluent_typed_codegen)
[![License](https://img.shields.io/crates/l/fluent_typed_codegen)](LICENSE)

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

[API documentation](https://docs.rs/fluent_typed_codegen/latest/fluent_typed_codegen/) ·
[Runnable Rust example](examples/minimal/README.md)

## Contents

- [Setup](#setup)
- [Configuration](#configuration)
- [Typed translation scopes](#typed-translation-scopes)
- [External storage and translation packs](#external-storage-and-translation-packs)
- [Numbers, plurals and RTL](#numbers-plurals-and-rtl)
- [Features and dependency boundaries](#features-and-dependency-boundaries)
- [Outputs and indexing](#outputs-and-indexing)
- [Framework extensions](#framework-extensions)
- [Custom build tooling](#custom-build-tooling)
- [Continuous integration](#continuous-integration)
- [License](#license)

## Setup

The declared minimum Rust version is 1.95.
Future releases may raise the compiler requirement; release notes will identify
the last version supporting the previous minimum.

### 1. Add the dependencies

In your application's `Cargo.toml`, enable generation in the build dependency
and the lightweight inclusion macro in the normal dependency:

```toml
[build-dependencies]
fluent_typed_codegen = { version = "0.1.3", default-features = false, features = ["build"] }

[dependencies]
fluent_typed_codegen = { version = "0.1.3", default-features = false }
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

Create `assets/localizations/localization.toml`:

```toml
translations-directory = "translations"
source-language = "en"
default-language = "en"
```

Create matching files for each language:

```text
assets/localizations/
├── localization.toml
└── translations/
    ├── en/presentation/hud.ftl
    └── es/presentation/hud.ftl
```

`en/presentation/hud.ftl`:

```ftl
title = Dashboard
# $name (String) - Person to greet.
greeting = Hello, { $name }!
```

`es/presentation/hud.ftl`:

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

fn main() {
    let translations = texts::Locale::En.load();
    println!("{}", translations.presentation().hud().msg_greeting("Ada"));
}
```

Run `cargo run` to print `Hello, Ada!`. Load `texts::Locale::Es` to use Spanish.
`cargo check` also runs generation, so rust-analyzer can index the API without
running the application. For a larger working example with checked external
loading and localized numbers, see [examples/minimal](examples/minimal/README.md).

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
Use [`from_cargo`](https://docs.rs/fluent_typed_codegen/latest/fluent_typed_codegen/fn.from_cargo.html)
for custom `Result`-based build-error handling.

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
| Whole language | `texts::Translations` | `texts::Locale::En.load()` |
| `presentation/` | `texts::Presentation` | `translations.presentation()` |
| `presentation/hud.ftl` | `texts::presentation::Hud` | `translations.presentation().hud()` |

Leaf types expose upstream message accessors through `Deref`; there is no public
`presentation::hud` module. Additional parameter and structured-result types
appear beside the leaf with its name as a prefix, such as `presentation::HudPrompt`.
Equal keys in different files remain independent and may have different types.
Clones share immutable catalogs through `Arc`.

### Loading and replacing translations

`Locale::load()` and `Translations::embedded(locale)` parse embedded FTL data.
`Translations::from_modules(locale, &[("presentation/hud.ftl", source), ...])`
checks complete path inventory, exact keys/variables/references and upstream typed
contracts before returning a snapshot. Prose-only edits work; missing/extra/
duplicate modules, removed variables and changed references fail. These methods
perform no filesystem reads: applications supply complete FTL strings, read
external files and decide when to replace a snapshot.
The application also owns UI updates. Language/module inventory or schema changes
require regeneration.

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
under `OUT_DIR`. `MODULES` records `(locale, relative module path, embedded source)`;
`CATALOG_ASSET_PATH` and `LANGUAGES_DIRECTORY` describe the definition and layout.
`ASSET_ROOT` is a build-time location, not a runtime storage requirement.

Bevy consumers additionally package the original definition TOML and preserve
its relative directory layout. The
[Bevy source integration](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/asset-sources.md)
uses this same checked parser through a named asset source, then updates resources
and bound text. Other applications can call `from_modules` directly.

Compatible prose updates need no new executable. Compiled locales, module paths
and typed contracts still require regeneration when changed. Embedded fallbacks
remain in the binary; external storage is not an external-only generation mode.

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
let translations = texts::Locale::Ru.load();
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
plural-rule implementation. The [tests](examples/minimal/src/tests.rs) also cover
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

## Features and dependency boundaries

| Feature selection | API and dependencies |
| --- | --- |
| `build` (default) | Discovery, validation, generation and the typed extension API. Use in build-dependencies. |
| `default-features = false`, no features | Only `translations!`; the crate has no dependencies. Use in normal dependencies. |

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
Import the runtime libraries as `fluent_typed` and `fluent_syntax` inside the
generated module, then include `concat!(env!("OUT_DIR"), "/translations.rs")`.
This approach does not require the normal macro dependency. Framework bridges
can instead re-export the runtime libraries.

## Outputs and indexing

All Cargo outputs stay in `OUT_DIR`, respecting the selected profile and target:

| Output | Purpose |
| --- | --- |
| `translations.rs` | `Translations`, `Locale`, groups and file types. |
| `locale_modules.rs` | Original sources and build-validated metadata. |
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
[`Extension`](https://docs.rs/fluent_typed_codegen/latest/fluent_typed_codegen/trait.Extension.html)
to generate an additional entrypoint with framework traits, attributes or
registration code. The plain tree is always generated unchanged.

| Hook or descriptor | Syntax types |
| --- | --- |
| `root_imports`, `scope_imports` | `Vec<syn::ItemUse>` |
| `type_attributes` | `Vec<syn::Attribute>` |
| `type_declaration` | Annotated `syn::ItemStruct` in, `syn::Item` out |
| `root_items` | `Vec<syn::Item>` |
| `Scope` | `syn::Path` and a sequence of `syn::Ident` accessor names |

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
git clone https://github.com/SDA-31/fluent_typed_codegen.git
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
development; external applications use the registry dependencies in Setup.

## Continuous integration

[CI workflow](.github/workflows/ci.yml) runs on pushes (including tags), pull
requests and manual dispatch. It checks this repository independently:

- Generator tests, doctests, macro-only dependency isolation and the generated
  consumer on Linux, Windows and macOS with stable Rust.
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
