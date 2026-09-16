//! Generate accessors and metadata in Cargo OUT_DIR or an explicit output directory.
use crate::{
	Extension, diagnostics, discovery::discover, message_types, metadata, render,
	settings::Settings, source, tree::Node, upstream,
};
use std::{
	env, fs,
	path::{Path, PathBuf},
	process::ExitCode,
};

/// Generate catalogs from a Cargo build script with readable failure output.
///
/// Return this value from `main`: validation errors are printed to stderr and
/// produce a failing exit code without a panic/backtrace. Use [`from_cargo`] when
/// the caller needs to handle the error itself. This does not terminate the process.
///
/// ```no_run
/// fn main() -> std::process::ExitCode {
///     fluent_typed_codegen::build()
/// }
/// ```
#[must_use = "return this exit code from build.rs main so generation failures stop the build"]
pub fn build() -> ExitCode {
	diagnostics::build_outcome(from_cargo(), &mut std::io::stderr().lock())
}

/// As [`build`], with an opt-in framework source extension.
#[must_use = "return this exit code from build.rs main"]
pub fn build_with(extension: &dyn Extension) -> ExitCode {
	diagnostics::build_outcome(from_cargo_with(extension), &mut std::io::stderr().lock())
}

/// Generate using the consuming package's Cargo.toml metadata and Cargo OUT_DIR.
///
/// Call from the consumer's `build.rs` using this crate as a build-dependency. Emits
/// Cargo rerun directives for the manifest, configuration and language tree.
///
/// # Errors
/// Returns a diagnostic if Cargo environment variables are absent, settings or
/// catalogs are invalid, or generation/file access fails. Propagate this failure
/// instead of including possibly incomplete generated output. Prefer [`build`]
/// for the standard build.rs entrypoint, without `expect` or panic output.
///
/// ```no_run
/// # fn main() -> Result<(), String> {
/// fluent_typed_codegen::from_cargo()?;
/// # Ok(())
/// # }
/// ```
pub fn from_cargo() -> Result<(), String> {
	cargo_generation(None)
}

/// As [`from_cargo`], also emitting the extension's entrypoint.
///
/// # Errors
/// Returns configuration, catalog, extension-name, unsupported extension-syntax
/// or I/O diagnostics. Extension-hook panics propagate to the caller.
pub fn from_cargo_with(extension: &dyn Extension) -> Result<(), String> {
	cargo_generation(Some(extension))
}

fn cargo_generation(extension: Option<&dyn Extension>) -> Result<(), String> {
	let package =
		PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR is unset")?);
	let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is unset")?);
	let manifest_path = package.join("Cargo.toml");
	println!("cargo::rerun-if-changed={}", manifest_path.display());
	let manifest = fs::read_to_string(&manifest_path).map_err(|error| error.to_string())?;
	let settings = Settings::from_manifest(&manifest)?;
	generate_inner(&package, &output, &settings, extension)
}

/// Generate from explicit paths, for custom build frontends and tests.
///
/// `package` locates relative [`Settings`] paths. `output` is created if missing;
/// put it in Cargo `OUT_DIR` or a tool-owned directory under `target/`, never in
/// source resources. Writes the named facade `translations.rs`, metadata in
/// `locale_modules.rs`, the checked loader's `validation.rs`, and upstream APIs/FTL
/// under `modules/<module path>/`. Persistent `inputs/<inventory hash>/` staging
/// files let upstream Cargo rerun dependencies remain valid between builds.
/// Each FTL file has its own key namespace. Generated modules require consumer-owned
/// `fluent-typed` and `fluent-syntax` dependencies, including embedded-only use. Framework
/// extensions may supply these through their own runtime re-exports. Language
/// directories and nested modules are discovered deterministically. Unrelated
/// output files, including previously generated extensions, are left untouched.
/// Existing owned outputs are overwritten; Cargo rerun directives are emitted to
/// stdout even when called outside a Cargo build script.
///
/// # Errors
/// Rejects invalid configuration, absent/noncanonical languages, symlinked catalog
/// entries, module/schema mismatches, duplicate keys within a file, ambiguous Rust
/// module names, unresolved/cyclic local references, invalid typed Fluent or I/O
/// failures. Cross-file Fluent references are not supported. Output is not
/// transactional; discard failed generation results.
pub fn generate(package: &Path, output: &Path, settings: &Settings) -> Result<(), String> {
	generate_inner(package, output, settings, None)
}

/// As [`generate`], with an additional framework entrypoint.
///
/// # Errors
/// Includes all [`generate`] errors, invalid or colliding extension names and
/// unsupported extension syntax. Extension-hook panics propagate to the caller.
pub fn generate_with(
	package: &Path,
	output: &Path,
	settings: &Settings,
	extension: &dyn Extension,
) -> Result<(), String> {
	generate_inner(package, output, settings, Some(extension))
}

fn generate_inner(
	package: &Path,
	output: &Path,
	settings: &Settings,
	extension: Option<&dyn Extension>,
) -> Result<(), String> {
	if let Some(extension) = extension {
		let filename = extension.filename();
		let path = Path::new(filename);

		if path.file_name().and_then(|name| name.to_str()) != Some(filename)
			|| !filename.ends_with(".rs")
			|| filename.contains(['\\', '#', ':'])
			|| matches!(
				filename,
				"translations.rs" | "locale_modules.rs" | "validation.rs"
			) {
			return Err(format!("invalid extension output filename: {filename:?}"));
		}
	}

	let package = package.canonicalize().map_err(|error| error.to_string())?;
	let sources = discover(&package, settings)?;
	let paths: Vec<_> = sources
		.modules
		.iter()
		.filter(|module| module.language == sources.configuration.source_language)
		.map(|module| module.path.clone())
		.collect();
	let tree = Node::build(&paths)?;
	render::validate_extension(&tree, extension)?;
	let config_path = settings.catalog_path(&package);
	let root = &sources.root;
	println!("cargo::rerun-if-changed={}", config_path.display());
	println!("cargo::rerun-if-changed={}", root.display());
	fs::create_dir_all(output).map_err(|error| error.to_string())?;
	let output = output.canonicalize().map_err(|error| error.to_string())?;

	fs::write(
		output.join("locale_modules.rs"),
		source::format(
			metadata::render(settings, &sources, &config_path)?,
			"locale_modules.rs",
		)?,
	)
	.map_err(|error| error.to_string())?;
	upstream::generate(&sources, &paths, &output)?;
	let message_types = message_types::discover(&paths, &output)?;
	message_types::validate(&tree, &message_types)?;
	// A single maintained checker serves discovery and generated checked APIs.
	let validation = include_str!("schema.rs").replacen("//!", "//", 1).replacen(
		"use fluent_syntax::{ast, parser};",
		"use super::fluent_syntax::{ast, parser};",
		1,
	);
	fs::write(output.join("validation.rs"), validation).map_err(|error| error.to_string())?;
	fs::write(
		output.join("translations.rs"),
		source::format(
			render::render(&tree, &sources, &message_types, None)?,
			"translations.rs",
		)?,
	)
	.map_err(|error| error.to_string())?;

	if let Some(extension) = extension {
		fs::write(
			output.join(extension.filename()),
			source::format(
				render::render(&tree, &sources, &message_types, Some(extension))?,
				extension.filename(),
			)?,
		)
		.map_err(|error| error.to_string())?;
	}

	Ok(())
}
