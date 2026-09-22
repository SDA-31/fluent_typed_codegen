# Changelog

## 0.1.4 — 2026-09-22

Documentation and examples refresh; no public API or parsing behavior changes.

- Clarify setup, explicit build-time generation and the relationship to fluent-typed.
- Demonstrate application-owned ICU formatting and plural-category selection.
- Document storage-independent `from_modules` loading, original FTL pack layout,
  generated metadata and input ownership, including generated Rustdoc.
- Add coverage for owned virtual sources and move example regression tests into
  `examples/minimal/tests/`, keeping the runnable example focused on API usage.

Earlier releases predate this changelog; their source is retained in Git tags.
