# Embedded selector experiment

Local experiment dated 2026-09-29 on `feat/embedded-selector-diagnostics`, based
on `c629a53`. This is a maintainer handoff, not a release or migration guide.
README and published-version documentation are intentionally unchanged while
the selector contract is evaluated. No version bump, push, tag or publication
is part of this experiment.

## Proposed calls

```rust,ignore
use texts::presentation::Hud as Interface;

let complete = texts::embed_manifest!();
let selected = texts::embed_manifest!(texts::presentation::Hud);
let compatible = texts::embed_manifest!(module = texts::presentation::Hud);
let hud = Interface::from_manifest(texts::Locale::En, &selected)?;
```

Leaf, group and root paths work. Paths may use `crate`, `self`, `super`, a leading
`::`, raw identifiers and aliases of parent modules. A trailing comma is allowed
for a selector. Ordinary imports and type aliases remain usable for constructors,
accessors and Bevy resources, including `Res<Interface>`.

The embedded selector must name the generated type in its generated namespace.
`embed_manifest!(Interface)` and `embed_manifest!(crate::Interface)` are rejected.
Strings, generic arguments and arbitrary expressions are rejected too. The
short form avoids needing an editor completion for the custom `module =` syntax;
that keyword itself has no new completion support.

## Implementation and limits

There is no longer a `Hud!` companion macro. Each generated namespace has one
dispatcher named `__embed_scope!`, with deferred include recipes for its types.
It is an implementation helper with crate visibility, not a completely private
macro: sibling call sites must still be able to resolve it. Internal names can
still appear in completion when their prefix is entered. Hover also shows the
internal `__embed_manifest` definition name alongside its public documentation.

The selector checks its argument in a `PhantomData<T>` type position, preserving
type-path completion without constructing a catalog. Macro expansion checks
known terminal names and dispatches through the supplied namespace. These are
compile-time token matches, not runtime iteration, type reflection or loading.
Equal type names in different namespaces use their own recipes.

Short names, unsupported syntax, unknown terminal names and known names used in
the wrong generated namespace get explicit `compile_error!` diagnostics. There
is a remaining boundary: a re-export retaining a known name in a foreign module,
such as `forwarded::Hud`, gets Rust's missing-`__embed_scope` path error. A missing
parent module likewise gets an ordinary resolution error. The macro cannot
inspect Rust's resolved type identity or detect every alias itself. These cases
must not silently select another catalog's bytes.

With no invocation, no FTL include expands. A selective invocation reads only
its scope's original files; full embedding includes all scopes. Build-time schema
generation still reads all configured translations. This does not change runtime
loading, Fluent parsing, the storage of an instantiated module or source lifetime.

## Verification

Checked using Rust 1.98.1 on Linux:

- All 42 generator tests and six active doctests passed; three illustrative
  doctests remain ignored as before. The expanded embedding regression also
  passed after its final additions.
- The embedding regression deletes original FTL after generation and restores
  only selected scopes. It runs executables and checks their payload sentinels
  with `opt-level=0`, LTO off, debug information, no stripping and
  `-Clink-dead-code=yes`. It covers leaf/group/root selection, both call forms,
  duplicate type names in separate namespaces, ordinary catalog aliases, parent
  module aliases, absolute/relative/raw paths, and invalid selectors. Invalid
  selectors are checked with every original FTL removed. Type-named macros fail
  to resolve under both their original and imported names.
- `cargo clippy --offline --all-targets -- -D warnings`, `cargo fmt --all --check`,
  `git diff --check` and Rustdoc with `-D warnings` passed.
- Checks with no default features and with only `manifest` passed. The standalone
  macro-only dependency tree remains empty.
- A separate rust-analyzer 0.3.3041 instance inspected actual generated output.
  Completion inside the short selector returned `Hud` as a struct (LSP kind 22)
  without `Hud!`. Hover returned the macro usage documentation. The bare alias
  produced the intended `macro-error` diagnostic from rust-analyzer itself.
- A standalone Bevy 0.19.1 consumer used the current runtime with this generator
  patched into both dependency graphs. Explicit leaf embedding, `Lazy` loading
  and a localized system with `Res<Interface>` succeeded; the unselected leaf
  and complete root were absent from the world. No runtime changes were needed.

Temporary editor and Bevy fixtures are respectively under
`/tmp/fluent-embedded-selector-editor-2026-09-29` (including `probe.py` and
`report.json`) and `/tmp/fluent-embedded-selector-bevy-2026-09-29`. They are local
verification artifacts, not package files. No CI or older-backend matrix was run.

## Before adopting this contract

Rejecting imported selector names is incompatible with the released embedding
API. Decide that tradeoff before updating versioned migration notes, changelogs
and public guides. Keep build and runtime generator dependencies aligned; the
experimental generated helpers require this branch's macro implementation.

The runtime's existing test
`examples/minimal/tests/catalog/macros.rs::typed_embedding_resolves_nested_modules_and_import_aliases`
still expects embedding through an imported type name. If this approach is
adopted, change that selector to a qualified generated path while retaining its
alias for catalog construction. Update the runtime's macro Rustdoc and embedding
guides at the same time. Those files and both main checkouts are unchanged here.
