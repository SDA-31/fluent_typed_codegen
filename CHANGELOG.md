# Changelog

## 0.2.0 (Unreleased) — runtime module loading

- Generate schema and typed accessors without mandatory embedded FTL. Explicit
  `embed_manifest!()` includes the whole set; `module = texts::ui::Menu` selects a
  leaf and `module = texts::Ui` selects a group. Generated paths and `use` aliases
  replace the earlier development snapshot's string selectors.
- Keep unselected FTL out of unoptimized binaries without relying on LTO or
  linker dead-code removal; regression tests compile with source files removed
  and inspect executable payloads.
- Add checked/unchecked leaf construction, standalone validation, module metadata,
  complete all-language loading, and typed errors with locale/path context.
- Add a shared immutable `LocalizationManifest` contract. Its std-only API keeps
  the no-default-feature dependency graph empty; optional `manifest` enables TOML.
- Add checked manifest constructors and integration hooks to assemble complete
  scopes from already parsed children without reparsing.
- **Migration:** remove generated `Translations::embedded`, `Locale::load` and
  source text from `MODULES`; use explicit data or manifest constructors. Root
  constructors now return `LoadError` instead of `String`. Manual output includes
  need the `__fluent_codegen` support alias. Extension `Scope` gains logical path
  and leaf metadata. Typed message accessors retain their signatures.
- Embedded macros are crate-local. Upstream input copies/reparsing remain; this
  change does not claim zero-copy loading or introduce fallback/cache policies.

## 0.1.4 — 2026-09-22

Documentation and examples refresh; no public API or parsing behavior changes.

- Clarify setup, explicit build-time generation and the relationship to fluent-typed.
- Demonstrate application-owned ICU formatting and plural-category selection.
- Document storage-independent `from_modules` loading, original FTL pack layout,
  generated metadata and input ownership, including generated Rustdoc.
- Add coverage for owned virtual sources and move example regression tests into
  `examples/minimal/tests/`, keeping the runnable example focused on API usage.

Earlier releases predate this changelog; their source is retained in Git tags.
