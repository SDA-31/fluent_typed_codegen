# Changelog

Notable changes are recorded here using
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html);
before 1.0, a minor release can introduce incompatible API changes.

## [Unreleased]

### Fixed

- Embedded module lookup builds a shared index on first use instead of scanning
  every embedded entry for each request. Large `read_modules` calls no longer
  spend quadratic time finding their source bytes; payloads remain borrowed.

## [0.2.0] - 2026-09-28

Follow the [migration from 0.1.4](docs/migration-0.2.md)
for dependency updates and before/after initialization examples.

### Added

- Checked leaf construction, optional unchecked construction, standalone
  validation and typed errors with module/locale context.
- Explicit file and embedded source contracts through `LocalizationManifest`;
  optional `manifest` support parses TOML without importing the generator.
- `load_all` and `load_all_unchecked` for complete sets of known languages.
- Typed `embed_manifest!` selection for a leaf, group or complete tree, including
  `use` aliases. Unselected FTL stays out of unoptimized binaries too.
- Metadata and integration hooks for assembling scopes from already parsed children.

### Changed

- **Breaking:** `Translations::from_modules` returns generated `LoadError`
  instead of `String`.
- **Breaking:** manual output includes require the `__fluent_codegen` support
  alias; extension `Scope` gains logical scope and leaf paths.
- Generation prepares a schema without mandatory FTL payloads. Applications
  explicitly choose storage, validation policy and module lifetimes.

### Removed

- **Breaking:** generated `Locale::load` and `Translations::embedded`;
  use explicit byte or manifest constructors.
- **Breaking:** embedded source text in `MODULES`; entries contain only locale
  and logical module path.

Typed message accessor signatures are unchanged. Upstream copies/reparsing remain;
this release does not introduce automatic fallback or caching.

## [0.1.4] - 2026-09-22

### Changed

- Clarify setup, explicit build-time generation and the relationship to fluent-typed.
- Demonstrate application-owned ICU formatting and plural-category selection.
- Document storage-independent `from_modules` loading, original FTL pack layout,
  generated metadata and input ownership, including generated Rustdoc.
- Add coverage for owned virtual sources and move example regression tests into
  `examples/minimal/tests/`, keeping the runnable example focused on API usage.

This documentation release changes no public API or parsing behavior.
Earlier releases predate this changelog; their source is retained in Git tags.

[Unreleased]: https://github.com/SDA-31/fluent_typed_codegen/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/SDA-31/fluent_typed_codegen/compare/v0.1.4...v0.2.0
[0.1.4]: https://github.com/SDA-31/fluent_typed_codegen/compare/v0.1.3...v0.1.4
