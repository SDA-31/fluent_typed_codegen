# Migrate from fluent_typed_codegen 0.1.4 to 0.2.1

This guide upgrades an application using the 0.1.4 API to 0.2.1.

The smallest migration keeps your full translation tree and typed message calls.
Only initialization, error handling and code that reads embedded metadata change.
Module-by-module loading is optional.

## 1. Update both dependency sections

Replace the generator entries in your application's Cargo.toml. Keep your
localization metadata, FTL layout and the two Fluent runtime dependencies:

```toml
[dependencies]
fluent_typed_codegen = { version = "0.2.1", default-features = false }
fluent-typed = { version = "0.9.0", default-features = false, features = ["langneg"] }
fluent-syntax = "0.12"

[build-dependencies]
fluent_typed_codegen = { version = "0.2.1", default-features = false, features = ["build"] }
```

Remove development Git/path overrides for this package when switching to the
registry. If you rename the dependency, keep the same alias in both sections.
Rust 1.95 remains the minimum; retain Cargo resolver 2 or 3.

Keep generation explicit in `build.rs`:

```rust
fn main() -> std::process::ExitCode {
    fluent_typed_codegen::build()
}
```

Keep `fluent_typed_codegen::translations!(pub mod texts);` in application code.
The macro includes prepared output; it does not run generation.

## 2. Replace implicit embedded initialization

Before, in 0.1.4, either call constructed a complete language:

```rust
let translations = texts::Locale::En.load();
// Equivalent old entrypoint:
let translations = texts::Translations::embedded(texts::Locale::En);
```

After, explicitly choose the embedded source and handle construction errors.
With the [quickstart's catalogs](../README.md#3-add-the-catalogs), this is a
complete replacement `src/main.rs`:

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), texts::LoadError> {
    let manifest = texts::embed_manifest!();
    let translations = texts::Translations::from_manifest(texts::Locale::En, &manifest)?;
    println!("{}", translations.presentation().hud().msg_greeting("Ada"));
    Ok(())
}
```

The macro includes all known languages' raw FTL; `from_manifest` parses only the
selected language. Generation alone embeds no FTL. Static embedded bytes remain
in the executable for its lifetime, even when parsed catalogs are dropped.
There is no compressor/decompressor callback.

For a smaller embedded source, declare a manifest with a relative schema selector:

```rust
texts::embed_manifest! {
    pub const HUD = presentation::Hud;
}
let hud = texts::presentation::Hud::from_manifest(texts::Locale::En, &HUD)?;
```

A leaf includes its file across known languages; a group such as
`Presentation` includes its descendant leaves. Selectors are relative schema
paths, without `texts::`. Strings, imported aliases and generic parameters are
not selectors. Catalog aliases remain usable for constructors.
Only the empty invocation and the declaration block are accepted; old selector
arguments must be moved into a constant declaration.
A HUD-only manifest cannot construct a root that also requires other modules.
Unselected FTL stays out of debug builds too, without relying on optimization.

## 3. Keep your storage or choose files

Existing `Translations::from_modules(locale, &pairs)` calls keep accepting
`(logical_path, FTL_text)` pairs. Paths such as `presentation/hud.ftl` are below
the language directory; pass every compiled module exactly once.
The return error changes from `String` to the generated `texts::LoadError`:

```rust
fn load(
    locale: texts::Locale,
    pairs: &[(&str, &str)],
) -> Result<texts::Translations, texts::LoadError> {
    texts::Translations::from_modules(locale, pairs)
}
```

If your application's boundary must keep returning `String`, convert there with
`.map_err(|error| error.to_string())`. `LoadError` also implements
`std::error::Error`, so it can propagate into `Box<dyn std::error::Error>`.

To use the filesystem helper, enable `features = ["manifest"]` on the **normal**
generator dependency. Replace `src/main.rs` with:

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = fluent_typed_codegen::LocalizationManifest::from_file(
        "assets/localizations/localization.toml",
    )?;
    let translations = texts::Translations::from_manifest(texts::Locale::En, &manifest)?;
    println!("{}", translations.presentation().hud().msg_greeting("Ada"));
    Ok(())
}
```

Run from the application's directory. `from_file` reads the TOML contract;
`from_manifest` reads and parses the requested FTL. Installed applications supply
their installed manifest path. Build-time `asset-root` is not a runtime root.
Encrypted, compressed or network data stays application-owned: supply readable
bytes to `Hud::new(locale, &bytes)` or decoded text pairs to `from_modules`.

Construction validates by default. `_unchecked` methods are safe Rust but skip
schema compatibility checks; they still parse UTF-8/Fluent and can leave message
calls unable to format. A separate `validate` call before checked construction
is unnecessary. See [loading recipes](loading.md) for independent leaf lifetimes,
validation-only use and loading every known language.

## 4. Update metadata and custom integrations

| 0.1.4 usage | 0.2.1 replacement |
| --- | --- |
| `MODULES: &[(&str, &str, &str)]` | `MODULES: &[(&str, &str)]`: locale and logical path only |
| Read FTL from the third tuple field | Obtain readable data from your storage or an explicit manifest |
| Manual `include!` with only Fluent imports | Also import `fluent_typed_codegen as __fluent_codegen` in the generated module |
| Construct or exhaustively destructure extension `Scope` | Account for `logical_path: String` and `module_path: Option<String>` |

Prefer the declaration macro. If manual inclusion is necessary, this is the
complete module declaration with canonical runtime dependencies:

```rust
pub mod texts {
    use fluent_syntax;
    use fluent_typed;
    use fluent_typed_codegen as __fluent_codegen;

    include!(concat!(env!("OUT_DIR"), "/translations.rs"));
}
```

Keep a normal generator dependency with defaults disabled even when manually
including output. Framework extensions still exchange typed Rust syntax and
must compile a consumer of their output after upgrading. The hidden
`__from_parts` hook assembles already parsed children; application code normally
uses the public constructors.

## 5. Verify the application

Run `cargo check` and your application tests. Check startup, every supported
locale and any external translation pack. Typed message accessor signatures,
full-tree getters, FTL module paths and number-formatting ownership remain the
same. Applications still own fallback and caching; this upgrade does not provide
automatic eviction or eliminate upstream input copying/reparsing.

[All changes](../CHANGELOG.md) · [Loading recipes](loading.md) · [Setup](../README.md#setup)
