# Embedded constant experiment

Local work on `feat/embedded-selector-diagnostics`, based on `c629a53`, dated
2026-09-29. This is a maintainer handoff, not a release guide. The package version
remains unchanged; embedding documentation follows the implemented contract.
No push, tag, publication or CI run is part of this experiment; the game and its submodule gitlinks are outside its scope.

## Implemented API

```rust,ignore
texts::embed_manifest! {
    /// HUD sources in all discovered languages.
    pub const HUD = presentation::Hud;
    const PRESENTATION = Presentation;
    const COMPLETE = Translations;
}

use texts::presentation::Hud as Interface;
let hud: Interface = Interface::from_manifest(texts::Locale::En, &HUD)?;
```

Every declared value has the existing `LocalizationManifest` type. There is no
conversion or second public manifest type. Visibility, documentation, conditional
attributes and multiple declarations are supported. A private localization module
can export `HUD` and `Interface` without exposing its generated tree. The
crate-local embedding macro can also be re-exported within the consuming crate.

Selectors are paths relative to the invoked macro's generated tree, without
`texts::`. A leaf selects one logical FTL file, a group selects its descendants,
and `Translations` selects the complete tree, always in every discovered language.
Application imports and identically named application modules do not change the
selection. Imported aliases, strings, generic arguments and qualified application
paths are rejected with selector diagnostics. This is schema selection, not
reflection over Rust type identity. Ordinary catalog imports, type aliases and
`Res<Interface>` remain usable.

Only two input forms are accepted: `embed_manifest!()` for the complete source
set and `embed_manifest! { ... }` for named constants. Old selector arguments,
including `module = ...` and bare qualified paths, produce a diagnostic pointing
to those two forms. There is no legacy selector dispatch or `Hud!` companion macro.

## Representation and lifetime

Generated declarations contain deferred `include_bytes!` recipes and a private,
local namespace of scope markers for type-position completion. Exact token
matching selects the recipe at compile time; there is no runtime scan of types.
Only the selected recipe expands. Merely declaring the generated translation tree
or a `cfg`-disabled embedded constant does not read or include FTL bytes.
Build-time schema generation still reads the configured translation sources.

`LocalizationManifest` has an internal static embedded variant. Its entries are
sorted by locale and logical module path during discovery; selecting a scope
preserves this order. Reads use binary search over that static slice and borrow
its bytes. No heap-allocated lookup index is retained for this variant. The
existing dynamic embedded constructor keeps its indexed behavior, including
arbitrary input order and first-duplicate-wins semantics.

Each declaration has one `LazyLock<CatalogConfig>`. The three owned configuration
fields initialize on first `config()` access and remain shared for the process
lifetime; reading bytes does not initialize them. This preserves the existing
`config() -> &CatalogConfig` API. Cloning the constant's value shares its static
metadata and payloads. Neither operation parses Fluent or loads catalogs.
Fluent parsing and catalog lifetime remain controlled by normal loading APIs.

As with other owning Rust constants, bind a manifest before retaining a borrow
from a method: `let manifest = &HUD; let config = manifest.config();`.
Immediate calls such as `Interface::from_manifest(locale, &HUD)` and
`LocalizationPlugin::<Translations, Lazy>::new(HUD)` work directly.

## Editor behavior

A separate rust-analyzer 0.3.3041 instance inspected real generated output:

- Once a selector identifier is started, completion offers only scope types and
  child namespaces from the matching generated branch. `Hud` appears as a struct;
  there is no `Hud!` completion. Application-only names are not selector choices.
- Hover documents the declaration syntax and its `LocalizationManifest` result.
- An imported alias used as a selector produces the intended `macro-error`
  diagnostic without reading FTL.
- **Empty completion remains limited:** immediately after `=` or a trailing `::`,
  this rust-analyzer version returns no suggestions. Typing the first letter
  enables completion. Preserving path punctuation with a token-munching prototype
  did not improve this. No editor extension is included.

Go-to-definition for declaration selectors targets generated scope markers, not
the catalog implementation. Macro hover can show the internal definition name.
The earlier per-namespace dispatchers and qualified-selector helpers are removed;
hover documents only the empty call and constant declarations.

## Verification

Local checks use Rust 1.98.1 on Linux; no MSRV or older-Bevy matrix is claimed.
The permanent embedding regression generates schema, removes original FTL, then
restores only the selected files. It compiles and runs consumers with `opt-level=0`,
LTO off, debug information, no stripping and `-Clink-dead-code=yes`, inspecting
binary and output sentinels. Coverage includes:

- Leaf, group and root declarations; multiple constants; raw identifiers;
  visibility, documentation and disabled `cfg` declarations.
- Private generated trees, exported constants and catalog aliases, re-exported
  macros, and conflicting names in application scope.
- Invalid selectors with every source FTL absent, plus rejection of type-named
  macro calls under their original and imported names.
- Empty invocation versus root-constant parity, duplicate leaf type names in
  different namespaces, and rejection of removed selector-argument calls before
  file access. Generated output contains no old namespace dispatchers.

Manifest tests cover first/last/missing static entries, borrowed payload pointers,
request ordering, lazy metadata initialization across threads and clones, and
unchanged dynamic embedded behavior. The engine-free example uses a root constant.

Checks at the original constant-implementation checkpoint:

- `cargo test --offline -- --test-threads=1`: 43 library tests and six active
  doctests passed; four illustrative doctests remain ignored.
- `cargo test --offline --no-default-features --features manifest -- --test-threads=1`:
  six tests and one active doctest passed.
- The standalone engine-free example passed all 11 tests and Clippy with warnings
  denied. Its normal dependency disables generation; its build dependency enables it.
- Library `cargo clippy --offline --all-targets -- -D warnings`, macro-only
  `cargo check --offline --no-default-features`, and Rustdoc with warnings denied
  passed. `cargo tree --offline --no-default-features` contains only this crate.
- A Bevy 0.19.1 standalone consumer accepted the constant directly as the existing
  manifest type, loaded only the requested leaf under `Lazy`, and ran a localized
  system with `Res<Interface>`. Unrequested leaf/root resources remained absent.
- The rust-analyzer probe confirmed the completion, hover and diagnostics described
  above against actual generated code.

Independent read-only review inspected all nine changed paths from merge base
`c629a53`, surrounding code, permanent tests and local editor/Bevy probes: **No
findings**. The reviewer did not modify files or rerun builds. Formatting and
`git diff --check` also passed. Remaining verification limits are the untested
Rust 1.95/older-Bevy matrix and the documented empty-position editor completion.

Temporary LSP and Bevy consumers are under
`/tmp/fluent-embedded-selector-editor-2026-09-29` (including `probe.py` and
`report.json`) and `/tmp/fluent-embedded-selector-bevy-2026-09-29`. They are local
verification artifacts, not package files.

## Integration boundary

This is not yet a release migration. Selector arguments have been intentionally
removed; consumers replace them with manifest constant declarations. The Bevy
companion branch `feat/embedded-manifest-constants` updates its tests, examples and
documentation to this same two-form contract. Runtime loading needs no adapter
changes. Keep build and runtime generator dependencies aligned: the generated
recipes require the matching macro implementation.

Package versions are unchanged for local development. Before release, select a
new generator version, update the Bevy dependency minimum, and verify the supported
compiler/backend matrix and registry-only consumers. No release is authorized here.

## Two-form API follow-up verification

The selector-removal follow-up passed 140 tests across the generator, runtime and
five example packages (45 generator, 11 engine-free, 84 Bevy/runtime examples),
plus ten active doctests. Tests ran offline and locked with two compilation jobs,
serial test execution and the workspace development optimization levels overridden
to zero. The permanent debug-binary inclusion regression passed with deleted
unselected FTL, including rejection of all removed argument forms before file IO.

The standalone rust-analyzer probe used freshly generated output from the normal
checkout. Hover assertions reject both `module =` and qualified expression
examples; relative selector completion and alias diagnostics passed again.
The empty-position completion limitation remains unchanged.

Independent read-only review covered both corrective diffs against their previous
checkpoints, the surrounding implementations, tests and documentation: **No
findings**. The reviewer did not edit files or repeat compiler/editor checks.

Clippy passed for all targets of the same seven packages with `-D warnings`.
Rustdoc passed for both libraries with warnings denied and the runtime's
`codegen,build` features enabled. Formatting and `git diff --check` passed;
README and packaged Rustdoc describe the same two-form API. The supported
compiler/backend matrix and registry-only release checks remain outside this
local correction.

## Complete documentation audit

A dedicated read-only reviewer checked all six tracked Markdown files, public
and generated Rustdoc, example sources, feature declarations and the published
`v0.2.1` macro. Constant recipes had incorrectly inherited registry 0.2.1 setup
instructions. They now require a matching source checkout; the versioned migration
keeps the published no-argument API. The main registry quickstart is unchanged.
The reviewer rechecked the corrections and reported no remaining findings.

Across both libraries, 24 Markdown files and 210 local links/anchors were checked.
Rustdoc built with warnings denied; all ten active doctests passed (six generator,
four runtime). The selective loading recipe was extracted from Markdown together
with the README Cargo/FTL/build-script blocks, built offline outside the enclosing
workspace, and printed the expected Spanish greeting. Its documented Cargo patch
resolved the same generator for build and runtime. `cargo package --list` confirms
all Rustdoc include files are packaged. Formatting and whitespace checks passed.

Probes live under `/tmp/localization-doc-audit-2026-09-29`; they are not repository
files. External link availability and the release compiler/backend matrix were
not rerun. No implementation behavior, package version, CI workflow or release
reference changed during this documentation audit.
