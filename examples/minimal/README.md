# Minimal typed localization in Rust

A complete Rust application using a generated translation API. The default greeting
and documentation are English; Spanish and Russian translations are included.
Catalogs live under `assets/localizations/translations/{en,es,ru}/` with matching
module paths. Generated files stay in Cargo OUT_DIR.

The dependency is deliberately renamed to `l10n` in both Cargo sections.
These paths are internal to this repository, not external installation paths;
for your application, use the [registry dependency setup](../../README.md#setup).

```toml
[dependencies]
l10n = { package = "fluent_typed_codegen", path = "../..", default-features = false }
fluent-typed = { version = "0.9.0", default-features = false, features = ["langneg"] }
fluent-syntax = "0.12"
fluent_typed_decimal = "0.1.0"

[build-dependencies]
l10n = { package = "fluent_typed_codegen", path = "../..", default-features = false, features = ["build"] }
```

The build script returns `l10n::build()`. Application code declares
`l10n::translations!(pub mod texts);` and borrows named translation scopes.
There is no handwritten OUT_DIR path in the consumer. The macro-only normal
dependency has no dependencies of its own; the build instance enables generation.
The two Fluent runtime dependencies retain their standard names.
Typed message accessors and Fluent resolution come from
[fluent-typed](https://github.com/human-solutions/fluent-typed); this crate adds
module discovery, namespaces and checked whole-language loading.

Clone the repository and run the example from its root:

```sh
git clone https://github.com/SDA-31/fluent_typed_codegen.git
cd fluent_typed_codegen
cargo run --manifest-path examples/minimal/Cargo.toml
cargo test --manifest-path examples/minimal/Cargo.toml
cargo clippy --manifest-path examples/minimal/Cargo.toml --all-targets -- -D warnings
cargo tree --manifest-path examples/minimal/Cargo.toml --edges normal
```

The executable prints the English HUD title, a greeting and a pluralized item
count in every language. The optional application dependency
[fluent_typed_decimal](https://github.com/SDA-31/fluent_typed_decimal) supplies
locale-formatted text and the plural category; it is not a generator dependency.
`numbers.ftl` declares two String arguments for the Decimal path and a separate
native Number selector. The test checks `1` versus visible `1.0`, Russian `few` /
`many`, Spanish output and native Fluent exact-number matching. The adapter's own
generated example additionally covers Arabic digits and Arabic plural categories.

Tests cover typed parameters, named nested scopes, a Rust-keyword leaf,
structured results named `presentation::HudPrompt` (without a public `hud`
module), inferred results, renamed macro imports, restricted visibility,
forwarded attributes and strict external loading.
No source files are modified by running the example.

Standalone clones need no sibling checkout or generator patch. The initial
dependency resolution creates a local ignored Cargo.lock; add `--locked --offline`
on later runs. In an enclosing workspace the package can be a member named
`fluent-codegen-example`, using that workspace's lockfile. An all-features workspace
check intentionally enables more than this consumer's normal feature graph.

## License

[MIT](LICENSE), independently of any consuming application.
