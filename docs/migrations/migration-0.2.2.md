# Migrate from fluent_typed_codegen 0.2.2 to 0.3.0

This upgrade uses fluent-typed 0.10.1. The minimum Rust version remains 1.95;
keep Cargo resolver 2 or 3 and explicit generation in build.rs.

## Update the consumer graph

```toml
[dependencies]
fluent_typed_codegen = { version = "0.3.0", default-features = false, features = ["runtime"] }

[build-dependencies]
fluent_typed_codegen = { version = "0.3.0", default-features = false, features = ["build"] }
```

Keep the existing build.rs call and translations! declaration. Retain the optional
manifest feature when using file-manifest factories. Bevy consumers use the
bevy_fluent_typed 0.5.0 facade instead of declaring these dependencies directly.
Remove direct `fluent-typed` and `fluent-syntax` dependencies used only by generated
translations. The `runtime` feature supplies both through version-pinned re-exports,
including language negotiation. For direct upstream calls, use
`fluent_typed_codegen::fluent_typed` or `fluent_typed_codegen::fluent_syntax`.
Manual `include!` modules import those re-exports instead of external crate names.

## Rename structured message accessors

Plain message accessors already use the `msg_` prefix. Structured `(Element)`
accessors now use it too:

```rust,ignore
// Before
let prompt = options.prompt();
// After
let prompt = options.msg_prompt();
```

The returned structured type and its fields are unchanged. Rebuild the generated
code rather than editing files under target/.

## Fix invalid translations exposed by stronger checks

Upstream checks every reachable message, attribute, term, term attribute and
function, including cycles and variables in referenced patterns. NUMBER is the
registered Fluent function. A message invalid in any locale may lose its accessor
with a build warning; fix that locale's FTL instead of removing the Rust call.
Checked external loads reject the same contract violations before publication.
Unchecked constructors still skip generated-contract checks.

## Pass fresh buffers by ownership

`new` and `new_unchecked` now accept `Into<Vec<u8>>`, matching fluent-typed.
Pass a buffer returned by a file read, download or decompressor by value:

```rust,ignore
let bytes = std::fs::read(path)?;
let hud = Hud::new(locale, bytes)?;
```

A Vec<u8> or String transfers its allocation. To retain a Vec, use
`Hud::new(locale, bytes.as_slice())`; a slice is copied into Fluent's owned source.
The former `Hud::new(locale, &bytes)` call with `&Vec<u8>` needs this explicit
slice conversion. The same rule applies to direct upstream constructors.

File manifests transfer acquired buffers automatically. Embedded and shared input
still require an owned copy. Validation and parsing allocate their own structures;
this removes an extra full-source copy when the caller supplies an owned buffer.

## Extension authors

Root, group and leaf wrappers now derive Debug as well as Clone. Remove a duplicate
Debug derive from Extension::type_attributes or a declaration wrapper, and remove
a conflicting manual Debug implementation. Preserve
both built-in attributes in Extension::type_declaration. Validation uses upstream's
public MESSAGE_CONTRACTS without injecting a private forwarding function.

## Verify the application

Run the consumer's cargo check --locked and its translation tests. Exercise a
structured accessor, file/embedded loading, compatible external prose edits and
rejection of an invalid candidate. The [minimal example](../../examples/minimal)
contains runnable coverage of generated structured output.

## Check generated names and locale directories

fluent-typed 0.10.1 reports generated-name collisions as build errors. Rename
conflicting messages or arguments named by the diagnostic. Locale directories
must be canonical language identifiers with two- or three-letter language
subtags; move shared data outside the translation tree. Hidden directories such
as `.history` are ignored. Symbolic links remain unsupported.
