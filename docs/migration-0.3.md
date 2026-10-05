# Migrate from fluent_typed_codegen 0.2.1 to 0.3.0

Typed message accessors and catalog constructors keep their signatures. Update
build configuration and selective embedding before rebuilding your application.

## Update both dependencies

```toml
[dependencies]
fluent_typed_codegen = { version = "0.3.0", default-features = false }

[build-dependencies]
fluent_typed_codegen = { version = "0.3.0", default-features = false, features = ["build"] }
```

Keep your existing `fluent-typed`, `fluent-syntax` and optional `manifest` feature.
Generation remains an explicit `fluent_typed_codegen::build()` call in `build.rs`.
The `translations!` declaration is unchanged. Remove development overrides when
switching to registry dependencies.

## Use one catalog path

Before:

```toml
[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

After:

```toml
[package.metadata.localization]
catalog = "assets/localizations/localization.toml"
```

The path is relative to the consuming package's `Cargo.toml`. `../` can locate
shared sources outside that package. `translations-directory` remains relative
to the TOML; its directory restrictions and legacy alias are unchanged.

For direct generator calls, remove `Settings::asset_root` and use
`Settings { catalog: "assets/localizations/localization.toml".into() }`.
Generated `ASSET_ROOT` is removed; `CATALOG_PATH` replaces `CATALOG_ASSET_PATH`
and retains the package-relative filesystem input. Engine asset roots, ZIP keys
and runtime installation paths are application responsibilities. Do not blindly
rename an old engine-address constant into a filesystem path.

Unknown configuration fields are ignored in both Cargo metadata and the TOML.
Recognized fields still require valid values. A leftover `asset-root` has no
effect: adjust `catalog` even if you keep extra metadata. TOML syntax must remain
valid, and both names of the translation-directory field cannot be supplied.

## Declare selective embedded manifests

Before, a selector was supplied as an expression argument:

```rust,ignore
let hud_only = texts::embed_manifest!(module = texts::presentation::Hud);
```

After, declare a named manifest at module scope:

```rust,ignore
texts::embed_manifest! {
    pub const HUD = presentation::Hud;
    const PRESENTATION = Presentation;
}
```

Keep ordinary catalog aliases in application code:

```rust,ignore
use texts::presentation::Hud as Interface;
let hud = Interface::from_manifest(texts::Locale::En, &HUD)?;
```

Each constant has type `LocalizationManifest`. Selectors are relative paths in
the generated schema, without the `texts::` prefix. Imported type aliases remain
usable for catalogs and resources; they are not selectors. Visibility,
attributes and several declarations in one block are supported. A private
localization module can re-export its manifest constants and catalog aliases.

Only two macro forms remain: `texts::embed_manifest!()` for the whole tree and
`texts::embed_manifest! { ... }` for constants. Selector expression arguments,
including `module = ...`, are removed. The generated leaf types no longer share
their names with callable helper macros.

Constants contain selected static source bytes, not parsed catalogs. Metadata is
initialized on first access; reading bytes borrows static payloads. Parsing still
happens when constructing a catalog. Unselected FTL remains absent from debug
binaries without relying on optimization or stripping. Selected static bytes
remain for the executable's lifetime even after a parsed catalog is dropped.

Run your application tests after regeneration. See the [loading recipes](loading.md)
for byte and file sources, and the [changelog](../CHANGELOG.md) for release details.
