# Loading translations in a Rust application

Start with the [README setup](../README.md#setup). Each recipe includes a complete
replacement for its `src/main.rs`; keep the same `build.rs`, manifest and English/Spanish FTL files.
Run `cargo run` from the application directory after choosing a recipe.

The build script generates types and checks the source files. Runtime code then
chooses which translations to load and when to drop them. Calling a message
accessor never opens a file or loads another module.

| What you want | Entry point |
| --- | --- |
| Small application with no runtime files | [Explicit embedding](#embed-one-module) or the README's whole-language example |
| A runtime directory described by TOML | [File manifest](#read-files-through-a-manifest) |
| Bytes from your own file/archive/network code | [One module from bytes](#load-one-module-from-bytes) |
| A complete language from your own source | [Logical path/text pairs](#load-a-complete-language-from-pairs) |
| Every compiled language in memory | [All-language loading](#load-every-language) |
| Large data set, modules loaded only when needed | [Explicit lazy loading](#load-and-release-a-module-on-demand) |
| Check a translation without keeping a catalog | [Validation](#validate-without-keeping-a-catalog) |
| A setup or loading error | [Troubleshooting](../README.md#troubleshooting) |

## Read files through a manifest

First replace only the `fluent_typed_codegen` entry under `[dependencies]` with:

```toml
fluent_typed_codegen = { version = "0.2.2", default-features = false, features = ["manifest"] }
```

Keep the `[build-dependencies]` entry unchanged. The `manifest` feature enables
TOML parsing in the application; enabling `build` in the build script does not
enable runtime TOML parsing.

Then use this `src/main.rs`:

```rust
use fluent_typed_codegen::LocalizationManifest;

fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = LocalizationManifest::from_file(texts::CATALOG_PATH)?;
    let hud = texts::presentation::Hud::from_manifest(texts::Locale::En, &manifest)?;
    println!("{}", hud.msg_greeting("Ada"));
    Ok(())
}
```

`from_file` reads the TOML and creates a `LocalizationManifest`, a description of
the source. It does not read any FTL or create a translation catalog. The next
line reads, validates and parses only `en/presentation/hud.ftl`.

To load a complete language instead, replace the `hud` and `println!` lines with:

```rust
let translations = texts::Translations::from_manifest(texts::Locale::En, &manifest)?;
println!("{}", translations.presentation().hud().msg_greeting("Ada"));
```

`CATALOG_PATH` retains the package-relative build setting. This recipe runs from
the consuming package directory. Paths passed to `from_file` are relative to the
process's current directory,
not the executable. Ship the TOML and translation tree together and pass their
installed location in a packaged application. The runtime directory may differ
from the build-time `catalog` location; module paths and compiled contracts must
still match. File reads here are synchronous. Your application owns any async I/O.

If you already have the TOML text, `LocalizationManifest::parse(text, origin)`
creates the same contract without I/O. `origin` is the manifest's file path,
including its filename, so relative translation paths resolve beside it.

## Load one module from bytes

This recipe needs no `manifest` feature. Replace the file read with your own
download, decryption or decompression step when needed; pass readable FTL bytes
to the constructor.

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("assets/localizations/translations/en/presentation/hud.ftl")?;
    let hud = texts::presentation::Hud::new(texts::Locale::En, &bytes)?;
    drop(bytes);
    println!("{}", hud.msg_greeting("Ada"));
    Ok(())
}
```

`Hud` owns its parsed data and does not retain a borrow of `bytes`. The current
upstream loader copies source text; this is not zero-copy parsing. Dropping the
input buffer after construction is safe. Other FTL modules remain unloaded.

## Load a complete language from pairs

This recipe also needs no `manifest` feature. The setup has one FTL file per
language, so one pair is its complete module set:

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string("assets/localizations/translations/en/presentation/hud.ftl")?;
    let translations = texts::Translations::from_modules(
        texts::Locale::En,
        &[(texts::presentation::Hud::PATH, source.as_str())],
    )?;
    drop(source);
    println!("{}", translations.presentation().hud().msg_greeting("Ada"));
    Ok(())
}
```

For a larger schema, add one `(logical_path, FTL_text)` pair for each module of
that language. Keys look like `presentation/hud.ftl`, without an `en/` prefix.
Missing, duplicate and unknown modules produce errors. Input order does not matter.

## Load every language

`load_all` accepts `(locale, logical_path, FTL_text)` triples and returns a
`HashMap<Locale, Translations>`. Supply the complete module set for **every
compiled locale**. For the README's English/Spanish setup:

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let english = std::fs::read_to_string("assets/localizations/translations/en/presentation/hud.ftl")?;
    let spanish = std::fs::read_to_string("assets/localizations/translations/es/presentation/hud.ftl")?;
    let languages = texts::Translations::load_all(&[
        (texts::Locale::En, texts::presentation::Hud::PATH, english.as_str()),
        (texts::Locale::Es, texts::presentation::Hud::PATH, spanish.as_str()),
    ])?;

    for locale in [texts::Locale::En, texts::Locale::Es] {
        let translations = &languages[&locale];
        println!("{}", translations.presentation().hud().msg_greeting("Ada"));
    }

    Ok(())
}
```

This keeps both languages parsed in memory. Use a single leaf or a single
language instead when retaining the entire set is too expensive.

## Embed one module

Embedding needs no `manifest` feature and performs no runtime filesystem reads.
The macro uses the manifest already processed by `build.rs`; it takes no runtime
path or `LocalizationManifest` value.

Use generator 0.2.2 in both normal and build dependencies for this recipe.

```rust
fluent_typed_codegen::translations!(pub mod texts);

texts::embed_manifest! {
    const HUD = presentation::Hud;
}

fn main() -> Result<(), texts::LoadError> {
    let hud = texts::presentation::Hud::from_manifest(texts::Locale::Es, &HUD)?;
    println!("{}", hud.msg_greeting("Ada"));
    Ok(())
}
```

The selector includes that leaf's raw bytes in **all compiled languages**. Only
Spanish is parsed here. Use `texts::embed_manifest!()` to include every leaf and
language. Inside the declaration block, `Presentation` selects its descendant
leaves and `Translations` selects the whole tree. Each constant has type
`LocalizationManifest` and can be exported from a private localization module.

The macro accepts only an empty invocation or a block of constant declarations.
Selectors are original schema paths relative to the generated tree, not imported
aliases. `use texts::presentation::Hud as HudTexts` still works for constructing
the catalog: `HudTexts::from_manifest(locale, &HUD)`.

Without a macro invocation, the generated schema embeds no FTL payload, even in
debug builds without optimization, LTO or linker dead-code removal. Selecting a
leaf expands includes only for that leaf; siblings need not exist when compiling
the prepared schema. The earlier build-script generation still needs the schema's
source files.

Macro expansion happens during compilation, even inside `if false`. Dropping
`hud` releases its parsed data when no clones remain; embedded static bytes stay
in the executable. Embedding cannot make a large translation payload disappear
on unload. The macro is available only inside the crate declaring `translations!`.

## Load and release a module on demand

In the engine-free API, lazy loading means your application calls a constructor
when a module is needed. There is no background loader or special `Lazy` type.
The example below uses the `manifest` dependency feature from the file recipe.

```rust
use fluent_typed_codegen::LocalizationManifest;
use texts::{Locale, presentation::Hud};

fluent_typed_codegen::translations!(pub mod texts);

struct Screen {
    manifest: LocalizationManifest,
    locale: Locale,
    hud: Option<Hud>,
}

impl Screen {
    fn open(&mut self) -> Result<(), texts::LoadError> {
        if self.hud.is_none() {
            self.hud = Some(Hud::from_manifest(self.locale, &self.manifest)?);
        }

        Ok(())
    }

    fn close(&mut self) {
        self.hud = None;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut screen = Screen {
        manifest: LocalizationManifest::from_file("assets/localizations/localization.toml")?,
        locale: Locale::En,
        hud: None,
    };

    // No FTL has been read yet.
    screen.open()?;

    if let Some(hud) = &screen.hud {
        println!("{}", hud.msg_greeting("Ada"));
    }

    screen.close();
    assert!(screen.hud.is_none());
    Ok(())
}
```

Keep optional leaves in application state and apply the same pattern to each
screen or task. A complete `Translations` value retains all of its modules, so
do not retain one when you want independent module lifetimes. Clones share parsed
resources: dropping your last clone releases them. A cache owned by the application
may keep them alive longer. For a locale switch, construct the replacement leaf
with the new locale before replacing the current one if you want to keep the
current text on failure. There is no automatic fallback or eviction policy.

## Validate without keeping a catalog

Validation is useful for checking a downloaded translation pack before saving
it. It still parses the input and uses temporary memory. This recipe needs no
`manifest` feature:

```rust
fluent_typed_codegen::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("assets/localizations/translations/en/presentation/hud.ftl")?;
    texts::presentation::Hud::validate(&bytes)?;
    println!("The HUD translation matches the compiled contract.");
    Ok(())
}
```

For a complete language, `Translations::validate_modules(&pairs)` also checks
the module inventory. Ordinary `new`, `from_modules`, `load_all` and
`from_manifest` already validate; do not add a separate validation call before
them unless you need that separate result. It would parse the input again.

If your application deliberately skips contract checks, use `new_unchecked`,
`from_modules_unchecked` or `load_all_unchecked`. These are safe Rust methods;
they still parse Fluent, and whole-language methods still require the exact
module inventory. Incompatible messages may later fail to format or cause an
accessor to panic. Manifest constructors always validate; to skip those checks,
use `manifest.read(...)` and the corresponding unchecked byte constructor.

## Choose storage separately from the schema

`MODULES` describes compiled `(locale, logical_path)` entries. It contains no
translation bytes. `Hud::PATH` is the logical path of that generated leaf.
These values help your own loader find entries in a cache, archive or service.

`LocalizationManifest::config()`, `file_path()` and `embedded_modules()` expose
the source contract. `read(locale, path)` returns readable bytes;
`read_modules(locale, paths)` reads exactly the paths requested. They do not parse
FTL. Use your own transport for encrypted, compressed or remote content; there
is no decompressor callback or built-in archive/network protocol.
