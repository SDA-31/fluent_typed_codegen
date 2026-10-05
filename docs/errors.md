## Typed errors

The standard `build.rs` remains unchanged: return `fluent_typed_codegen::build()`
from `main() -> std::process::ExitCode`. It prints readable diagnostics and returns
failure without a panic. Use `from_cargo()` when the caller needs to handle failures.

```no_run
fn main() -> Result<(), fluent_typed_codegen::BuildError> {
    fluent_typed_codegen::from_cargo()
}
```

`from_cargo`, `from_cargo_with`, `generate` and `generate_with` return `BuildError`.
Match variants to distinguish missing Cargo environment, configuration failures,
I/O, catalog/module mismatches, schema/reference problems and generation failures.
I/O errors retain the operation, native filesystem path and `std::io::Error`.
Upstream failures retain their module and original `fluent_typed::BuildError`.

```rust
use fluent_typed_codegen::BuildError;

fn report(error: &BuildError) {
    match error {
        BuildError::Io { operation, path, source } => {
            eprintln!("{operation} {} failed: {:?}", path.display(), source.kind());
        }
        BuildError::ModuleMismatch(details) => {
            eprintln!("{}: missing {:?}, extra {:?}", details.locale, details.missing, details.extra);
        }
        _ => eprintln!("{error}"),
    }
}
```

`Settings::from_manifest` and `CatalogConfig::parse` return `ConfigError`.
Its `InvalidField` variant identifies the field and distinguishes `Missing`,
`WrongType` and `Empty`; `InvalidPath` preserves the original `PathBuf` and a
`PathError` reason. Unknown parameters remain ignored. Invalid TOML retains the
original parser error and span. No filesystem access occurs in these parsers.

`LocalizationManifest` reports `ManifestError::Config { source }` for configuration
and logical-path failures, `RequiresLoader { origin }` for virtual addresses,
`Io` for filesystem failures, and `MissingModule` for absent embedded entries.
The error types implement `Display` and `std::error::Error`; `source()` exposes
underlying parser, configuration, I/O and upstream errors where available.

Applications using `?` with a compatible error type continue to propagate failures.
Replace explicitly declared `Result<_, String>` with the appropriate typed error,
or convert deliberately with `map_err(|error| error.to_string())` at an application
boundary. These changes concern configuration and host generation; generated
catalog constructors retain their existing `LoadError` API. Build-only types and
syntax dependencies remain behind `build`; macro-only consumers need no dependencies.
