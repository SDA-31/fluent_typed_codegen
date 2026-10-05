//! Deterministic discovery and complete per-module contract checks.
use super::{references, settings::Settings};
use crate::{BuildError, CatalogConfig, IoOperation, ModuleMismatch, build_io, schema};
use fluent_typed::prelude::LanguageIdentifier;
use std::path::{Path, PathBuf};

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
pub(super) fn discover(package: &Path, settings: &Settings) -> Result<CatalogSources, BuildError> {
	settings.validate()?;
	let config_path = settings.catalog_path(package);
	let configuration =
		CatalogConfig::parse(&read(&config_path)?).map_err(|source| BuildError::Config {
			path: Some(config_path.clone()),
			source,
		})?;
	let mut root = config_path
		.parent()
		.expect("package-joined catalog has a parent")
		.to_owned();

	for component in configuration.languages_directory.components() {
		root.push(component);
		let metadata = build_io::symlink_metadata(&root)?;

		if metadata.file_type().is_symlink() {
			return Err(BuildError::Symlink { path: root.clone() });
		}
	}

	let mut languages = Vec::new();

	for entry in build_io::read_dir(&root)? {
		let entry =
			entry.map_err(|source| BuildError::io(IoOperation::ReadDirectory, &root, source))?;
		let kind = entry
			.file_type()
			.map_err(|source| BuildError::io(IoOperation::Metadata, &entry.path(), source))?;

		if kind.is_symlink() {
			return Err(BuildError::Symlink { path: entry.path() });
		}

		if !kind.is_dir() {
			continue;
		}

		let name = entry
			.file_name()
			.into_string()
			.map_err(|_| BuildError::NonUtf8Path { path: entry.path() })?;
		let id =
			name.parse::<LanguageIdentifier>()
				.map_err(|source| BuildError::InvalidLocale {
					path: entry.path(),
					locale: name.clone(),
					source,
				})?;

		if id.to_string() != name {
			return Err(BuildError::NonCanonicalLocale {
				path: entry.path(),
				locale: name,
				canonical: id.to_string(),
			});
		}

		languages.push(name);
	}

	languages.sort();

	for configured in [
		&configuration.source_language,
		&configuration.default_language,
	] {
		if !languages.contains(configured) {
			return Err(BuildError::MissingLocale {
				locale: configured.clone(),
				root: root.clone(),
			});
		}
	}

	let reference_root = root.join(&configuration.source_language);
	let paths = discover_modules(&reference_root, &reference_root)?;

	if paths.is_empty() {
		return Err(BuildError::EmptyCatalog {
			root: reference_root.clone(),
			locale: configuration.source_language.clone(),
		});
	}

	let mut reference = Vec::new();

	for path in &paths {
		let absolute = reference_root.join(path);
		let source = read(&absolute)?;

		let contract = schema::schema(&source).map_err(|source| BuildError::Schema {
			path: absolute.clone(),
			locale: configuration.source_language.clone(),
			source,
		})?;
		references::validate(&contract, &absolute)?;

		reference.push(source);
	}

	let mut modules = Vec::new();

	for language in &languages {
		let locale_root = root.join(language);
		let locale_paths = discover_modules(&locale_root, &locale_root)?;

		if locale_paths != paths {
			return Err(BuildError::ModuleMismatch(Box::new(ModuleMismatch {
				locale: language.clone(),
				source_language: configuration.source_language.clone(),
				locale_root,
				reference_root: reference_root.clone(),
				missing: paths
					.iter()
					.filter(|p| locale_paths.binary_search(p).is_err())
					.cloned()
					.collect(),
				extra: locale_paths
					.iter()
					.filter(|p| paths.binary_search(p).is_err())
					.cloned()
					.collect(),
			})));
		}

		for (path, expected) in paths.iter().zip(&reference) {
			let absolute = locale_root.join(path);
			let source = read(&absolute)?;
			schema::validate(&source, expected).map_err(|source| BuildError::Schema {
				path: absolute.clone(),
				locale: language.clone(),
				source,
			})?;

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

fn discover_modules(root: &Path, directory: &Path) -> Result<Vec<String>, BuildError> {
	let mut files = Vec::new();

	for entry in build_io::read_dir(directory)? {
		let entry = entry
			.map_err(|source| BuildError::io(IoOperation::ReadDirectory, directory, source))?;
		let path = entry.path();

		if entry
			.file_type()
			.map_err(|source| BuildError::io(IoOperation::Metadata, &path, source))?
			.is_symlink()
		{
			return Err(BuildError::Symlink { path });
		}

		if path.is_dir() {
			files.extend(discover_modules(root, &path)?);
		} else if path.extension().is_some_and(|extension| extension == "ftl") {
			let relative =
				path.strip_prefix(root)
					.map_err(|source| BuildError::PathOutsideRoot {
						path: path.clone(),
						root: root.into(),
						source,
					})?;
			files.push(BuildError::utf8(relative)?.replace('\\', "/"));
		}
	}

	files.sort();
	Ok(files)
}

fn read(path: &Path) -> Result<String, BuildError> {
	build_io::read_to_string(path)
}
