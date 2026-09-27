# Minimal typed localization in Rust

This example targets the unreleased `feat/runtime-module-loading` branch.

A complete Rust application using a generated translation API. The default greeting
and documentation are English; Spanish and Russian translations are included.
Catalogs live under `assets/localizations/translations/{en,es,ru}/` with matching
module paths. Generated files stay in Cargo OUT_DIR.

Start with [main.rs](src/main.rs) for typed calls and ICU formatting, then
[build.rs](build.rs) for the explicit generation step. Regression checks live
separately in [tests/unit/catalog.rs](tests/unit/catalog.rs).

The dependency is deliberately renamed to `l10n` in both Cargo sections.
These paths are internal to this repository, not external installation paths;
for your application, use the [complete application setup](../../README.md#setup).
For file loading or loading one module on demand, use the separate
[loading recipes](../../docs/loading.md); each provides a complete `main.rs`.

```toml
[dependencies]
l10n = { package = "fluent_typed_codegen", path = "../..", default-features = false }
fluent-typed = { version = "0.9.0", default-features = false, features = ["langneg"] }
fluent-syntax = "0.12"
icu_decimal = { version = "2.3", features = ["alloc"] }
icu_locale_core = "2.3"
icu_plurals = "2.3"

[build-dependencies]
l10n = { package = "fluent_typed_codegen", path = "../..", default-features = false, features = ["build"] }
```

The build script returns `l10n::build()`. Application code declares
`l10n::translations!(pub mod texts);`, explicitly invokes `texts::embed_manifest!()`,
and constructs a checked language with `Translations::from_manifest`. It then
borrows named translation scopes.
There is no handwritten OUT_DIR path in the consumer. The macro-only normal
dependency has no dependencies of its own; the build instance enables generation.
The two Fluent runtime dependencies retain their standard names.
Typed message accessors and Fluent resolution come from
[fluent-typed](https://github.com/human-solutions/fluent-typed); this crate adds
module discovery, namespaces and checked leaf/whole-language loading.

Clone the repository and run the example from its root:

```sh
git clone --branch feat/runtime-module-loading https://github.com/SDA-31/fluent_typed_codegen.git
cd fluent_typed_codegen
git checkout 5761d2843d80c5c4bbd6328d3037de61df398fdd
cargo run --manifest-path examples/minimal/Cargo.toml
cargo test --manifest-path examples/minimal/Cargo.toml
cargo clippy --manifest-path examples/minimal/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path examples/minimal/Cargo.toml --edges normal
```

The checkout pins the same tested, unreleased API as the README setup. It
predates 0.2.0's typed embedded selectors. To try typed leaf/group selection,
keep your current local checkout and follow the
[embedded recipe](../../docs/loading.md#embed-one-module); do not switch it to
the older revision above. The example's no-argument embedding works in both.
Run these commands in the clone, not in your application's directory. Installing the
published 0.1.4 crate does not install this API. To inspect matching API reference
pages locally, run `cargo doc --open` from the clone.

The executable prints the English HUD title, a greeting and a pluralized item
count in every language. The application uses
[ICU4X DecimalFormatter](https://docs.rs/icu_decimal/latest/icu_decimal/struct.DecimalFormatter.html)
and [PluralRules](https://docs.rs/icu_plurals/latest/icu_plurals/struct.PluralRules.html)
directly. ICU is an example dependency, not a generator dependency. Both services
receive the same Decimal, preserving its visible precision; the small `plural_key`
match only translates ICU's enum into a Fluent String selector, not plural rules.
`numbers.ftl` declares two String arguments for the Decimal path and a separate
native Number selector. The test checks `1` versus visible `1.0`, Russian `few` /
`many`, Spanish output and native Fluent exact-number matching. For repeated
rendering, retain the formatter and rules per locale rather than recreate them
per message. Apply any rounding once before passing a value to both services;
respect ICU's documented input/operand limits when accepting arbitrary precision.

## Reusing catalogs and formatters

The [compact ICU walkthrough](../../README.md#icu-example)
shows setup and a numeric argument separately. The input is a number
(`Decimal::from(22)`), not a string parsed from localized display text.

The generated `texts::Translations`, `DecimalFormatter` and `PluralRules` can be
kept in application state or cached by locale/options. Together they produce an
ordinary String. When the language changes, switch the catalog and services
together; recompute strings when their values, language or formatting settings
change. The application decides when to do this and how to use the result.

## Verification

Tests cover typed parameters, named nested scopes, a Rust-keyword leaf,
structured results named `presentation::HudPrompt` (without a public `hud`
module), inferred results, renamed macro imports, restricted visibility,
forwarded attributes, checked/unchecked leaf loading, strict inventories, all-language
construction, shared ready-leaf assembly and strict external loading.
No source files are modified by running the example.
The test module is loaded only by `cargo test`; it shares the application's small
plural-category helper without exposing a new public example API.

Standalone clones need no sibling checkout or generator patch. The initial
dependency resolution creates a local ignored Cargo.lock; add `--locked --offline`
on later runs. In an enclosing workspace the package can be a member named
`fluent-codegen-example`, using that workspace's lockfile. An all-features workspace
check intentionally enables more than this consumer's normal feature graph.

## License

[MIT](LICENSE), independently of any consuming application.
