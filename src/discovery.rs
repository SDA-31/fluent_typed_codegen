//! Deterministic discovery and complete per-module contract checks.
use super::{diagnostics, references, settings::Settings};
use crate::{CatalogConfig, schema};
use fluent_typed::prelude::LanguageIdentifier;
use std::{
	fs,
	path::{Path, PathBuf},
};

pub(super) struct SourceModule {
	pub language: String,
	pub path: String,
	pub absolute: PathBuf,
}

/// Checked inventory ordered by language, then relative FTL path.
/// All languages share the source language's module paths and reference schema.
pub(super) struct CatalogSources {
	pub modules: Vec<SourceModule>,
	pub configuration: CatalogConfig,
	pub root: PathBuf,
}

/// Discover locale directories and FTL files, rejecting symlinks within the
/// language tree and validating per-language parity.
/// Upstream argument-type validation runs later during generation.
pub(super) fn discover(package: &Path, settings: &Settings) -> Result<CatalogSources, String> {
	settings.validate()?;
	let config_path = settings.catalog_path(package);
	let configuration = CatalogConfig::parse(&read(&config_path)?)
		.map_err(|error| format!("{}: {error}", config_path.display()))?;
	let mut root = config_path
		.parent()
		.ok_or("catalog must have a parent directory")?
		.to_owned();

	for component in configuration.languages_directory.components() {
		root.push(component);
		let metadata =
			fs::symlink_metadata(&root).map_err(|error| format!("{}: {error}", root.display()))?;

		if metadata.file_type().is_symlink() {
			return Err(format!(
				"symbolic links inside catalogs are unsupported: {}",
				root.display()
			));
		}
	}

	let mut languages = Vec::new();

	for entry in fs::read_dir(&root).map_err(|error| format!("{}: {error}", root.display()))? {
		let entry = entry.map_err(|error| error.to_string())?;
		let kind = entry.file_type().map_err(|error| error.to_string())?;

		if kind.is_symlink() {
			return Err(format!(
				"symbolic links inside catalogs are unsupported: {}",
				entry.path().display()
			));
		}

		if !kind.is_dir() {
			continue;
		}

		let name = entry
			.file_name()
			.into_string()
			.map_err(|_| "locale directory must be UTF-8")?;
		let id = name
			.parse::<LanguageIdentifier>()
			.map_err(|error| format!("invalid language directory {name:?}: {error}"))?;

		if id.to_string() != name {
			return Err(format!(
				"language directory {name:?} must use canonical spelling {id:?}"
			));
		}

		languages.push(name);
	}

	languages.sort();

	for configured in [
		&configuration.source_language,
		&configuration.default_language,
	] {
		if !languages.contains(configured) {
			return Err(format!(
				"configured language {configured:?} has no directory in {}",
				root.display()
			));
		}
	}

	let reference_root = root.join(&configuration.source_language);
	let paths = discover_modules(&reference_root, &reference_root)?;

	if paths.is_empty() {
		return Err(format!(
			"no .ftl modules in source language {}",
			configuration.source_language
		));
	}

	let mut reference = Vec::new();

	for path in &paths {
		let absolute = reference_root.join(path);
		let source = read(&absolute)?;

		let contract =
			schema::schema(&source).map_err(|error| format!("{}: {error}", absolute.display()))?;
		references::validate(&contract)
			.map_err(|error| format!("{}: {error}", absolute.display()))?;

		reference.push(source);
	}

	let mut modules = Vec::new();

	for language in &languages {
		let locale_root = root.join(language);
		let locale_paths = discover_modules(&locale_root, &locale_root)?;

		if locale_paths != paths {
			return Err(diagnostics::module_mismatch(
				language,
				&configuration.source_language,
				&locale_root,
				&reference_root,
				&locale_paths,
				&paths,
			));
		}

		for (path, expected) in paths.iter().zip(&reference) {
			let absolute = locale_root.join(path);
			let source = read(&absolute)?;
			schema::validate(&source, expected)
				.map_err(|error| format!("{}: {error}", absolute.display()))?;
			modules.push(SourceModule {
				language: language.clone(),
				path: path.clone(),
				absolute,
			});
		}
	}

	Ok(CatalogSources {
		modules,
		configuration,
		root,
	})
}

fn discover_modules(root: &Path, directory: &Path) -> Result<Vec<String>, String> {
	let mut files = Vec::new();

	for entry in
		fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
	{
		let entry = entry.map_err(|error| error.to_string())?;
		let path = entry.path();

		if entry
			.file_type()
			.map_err(|error| error.to_string())?
			.is_symlink()
		{
			return Err(format!(
				"symbolic links inside catalogs are unsupported: {}",
				path.display()
			));
		}

		if path.is_dir() {
			files.extend(discover_modules(root, &path)?);
		} else if path.extension().is_some_and(|extension| extension == "ftl") {
			files.push(
				path.strip_prefix(root)
					.map_err(|error| error.to_string())?
					.to_str()
					.ok_or("module path must be UTF-8")?
					.replace('\\', "/"),
			);
		}
	}

	files.sort();
	Ok(files)
}

fn read(path: &Path) -> Result<String, String> {
	fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}
