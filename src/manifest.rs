//! Immutable source contracts, independent of any engine or loader.
use crate::{CatalogConfig, ConfigError, ConfigField, ManifestError, PathError, paths};
use std::{
	borrow::Cow,
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
	sync::{Arc, LazyLock, OnceLock},
};

type EmbeddedModules = &'static [(&'static str, &'static str, &'static [u8])];
type EmbeddedIndex = HashMap<&'static str, HashMap<&'static str, &'static [u8]>>;

/// A source contract for readable FTL; creating one never parses FTL.
///
/// File contracts retain configuration and its origin. Embedded contracts refer
/// to explicitly included bytes. There is no parsed-catalog cache or loaded state.
#[derive(Clone, Debug)]
pub struct LocalizationManifest {
	source: Source,
}

#[derive(Clone, Debug)]
enum Source {
	File {
		path: PathBuf,
		config: CatalogConfig,
	},
	Embedded {
		config: CatalogConfig,
		modules: EmbeddedModules,
		index: Arc<OnceLock<EmbeddedIndex>>,
	},
	StaticEmbedded {
		config: &'static LazyLock<CatalogConfig>,
		modules: EmbeddedModules,
	},
}

impl LocalizationManifest {
	/// Read only the TOML manifest, retaining its path for later module reads.
	///
	/// Relative paths use the process's current directory. Pass the installed
	/// manifest path in a packaged application; the build-time Cargo catalog setting
	/// is not consulted by this method.
	///
	/// # Errors
	/// Returns I/O or configuration diagnostics. FTL files are not accessed here.
	#[cfg(feature = "manifest")]
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
		let path = path.as_ref();
		let source = fs::read_to_string(path).map_err(|source| ManifestError::Io {
			path: path.into(),
			source,
		})?;
		Self::parse(&source, path)
	}

	/// Parse already obtained TOML with an explicit origin, without filesystem I/O.
	///
	/// The origin identifies the TOML file. It is retained verbatim, so an engine
	/// integration can interpret its own virtual address and avoid filesystem reads.
	///
	/// # Errors
	/// Returns TOML or configuration diagnostics.
	#[cfg(feature = "manifest")]
	pub fn parse(source: &str, origin: impl Into<PathBuf>) -> Result<Self, ManifestError> {
		let config =
			CatalogConfig::parse(source).map_err(|source| ManifestError::Config { source })?;
		Ok(Self::from_config(config, origin))
	}

	/// Describe a source from explicit configuration and its origin without I/O.
	/// Relative module paths are checked before engine-free filesystem reads.
	pub fn from_config(config: CatalogConfig, origin: impl Into<PathBuf>) -> Self {
		Self {
			source: Source::File {
				path: origin.into(),
				config,
			},
		}
	}

	/// Original manifest location for a file-backed contract.
	pub fn file_path(&self) -> Option<&Path> {
		match &self.source {
			Source::File { path, .. } => Some(path),
			_ => None,
		}
	}

	/// Explicitly embedded entries, for inspection or host-owned loading.
	pub fn embedded_modules(&self) -> Option<EmbeddedModules> {
		match &self.source {
			Source::Embedded { modules, .. } | Source::StaticEmbedded { modules, .. } => {
				Some(*modules)
			}
			_ => None,
		}
	}

	/// Configuration describing the source, independent of loaded modules.
	pub fn config(&self) -> &CatalogConfig {
		match &self.source {
			Source::File { config, .. } | Source::Embedded { config, .. } => config,
			Source::StaticEmbedded { config, .. } => config,
		}
	}

	/// Obtain one readable FTL module without parsing it or reading any siblings.
	///
	/// File data is owned; embedded data is borrowed. Locale and module are logical
	/// paths, not storage addresses. Engine integrations should use their own I/O.
	/// Generated embedded constants use sorted static entries directly, without
	/// allocating an index. Their configuration is initialized once on `config()`.
	/// Expression-based embedding indexes entry metadata on its first read; clones
	/// share that index. Neither path copies or parses the embedded FTL payloads.
	///
	/// # Errors
	/// Rejects invalid paths, missing embedded entries and I/O failures.
	pub fn read(&self, locale: &str, path: &str) -> Result<Cow<'_, [u8]>, ManifestError> {
		validate_request(locale, path)?;

		match &self.source {
			Source::File {
				path: origin,
				config,
			} => {
				if origin.to_string_lossy().contains("://") {
					return Err(ManifestError::RequiresLoader {
						origin: origin.clone(),
					});
				}

				paths::validate_relative(
					&config.languages_directory,
					ConfigField::TranslationsDirectory,
				)
				.map_err(|source| ManifestError::Config { source })?;

				let path = origin
					.parent()
					.unwrap_or_else(|| Path::new(""))
					.join(&config.languages_directory)
					.join(locale)
					.join(path);
				fs::read(&path)
					.map(Cow::Owned)
					.map_err(|source| ManifestError::Io { path, source })
			}
			Source::Embedded { modules, index, .. } => index
				.get_or_init(|| {
					let mut index = EmbeddedIndex::new();

					for &(language, module, bytes) in *modules {
						index
							.entry(language)
							.or_default()
							.entry(module)
							.or_insert(bytes);
					}

					index
				})
				.get(locale)
				.and_then(|modules| modules.get(path))
				.map(|bytes| Cow::Borrowed(*bytes))
				.ok_or_else(|| ManifestError::MissingModule {
					locale: locale.into(),
					path: path.into(),
				}),
			Source::StaticEmbedded { modules, .. } => modules
				.binary_search_by(|&(language, module, _)| (language, module).cmp(&(locale, path)))
				.map(|index| Cow::Borrowed(modules[index].2))
				.map_err(|_| ManifestError::MissingModule {
					locale: locale.into(),
					path: path.into(),
				}),
		}
	}

	/// Read exactly the requested paths, preserving their order for inspection.
	///
	/// # Errors
	/// Returns the first requested module's source error.
	#[allow(clippy::type_complexity)]
	pub fn read_modules(
		&self,
		locale: &str,
		paths: &[&str],
	) -> Result<Vec<(String, Cow<'_, [u8]>)>, ManifestError> {
		paths
			.iter()
			.map(|path| self.read(locale, path).map(|bytes| ((*path).into(), bytes)))
			.collect()
	}

	/// Construct the explicitly expanded build recipe without parsing TOML or FTL.
	#[doc(hidden)]
	pub fn __embedded(recipe: (&'static str, &'static str, &'static str, EmbeddedModules)) -> Self {
		let (source_language, default_language, directory, modules) = recipe;
		let config = CatalogConfig {
			source_language: source_language.into(),
			default_language: default_language.into(),
			languages_directory: directory.into(),
		};
		Self {
			source: Source::Embedded {
				config,
				modules,
				index: Arc::new(OnceLock::new()),
			},
		}
	}

	/// Construct a generated constant from unique entries sorted by locale and path.
	/// Configuration metadata is shared across uses; no runtime lookup index is kept.
	#[doc(hidden)]
	pub const fn __embedded_static(
		modules: EmbeddedModules,
		config: &'static LazyLock<CatalogConfig>,
	) -> Self {
		Self {
			source: Source::StaticEmbedded { config, modules },
		}
	}
}

fn validate_request(locale: &str, path: &str) -> Result<(), ManifestError> {
	paths::validate_relative(Path::new(locale), ConfigField::Locale)
		.map_err(|source| ManifestError::Config { source })?;
	paths::validate_relative(Path::new(path), ConfigField::Module)
		.map_err(|source| ManifestError::Config { source })?;

	if locale == "." || locale.contains('/') {
		return Err(ManifestError::Config {
			source: ConfigError::InvalidPath {
				field: ConfigField::Locale,
				path: locale.into(),
				reason: PathError::ExpectedLocaleComponent,
			},
		});
	}

	if !path.ends_with(".ftl") {
		return Err(ManifestError::Config {
			source: ConfigError::InvalidPath {
				field: ConfigField::Module,
				path: path.into(),
				reason: PathError::ExpectedFtl,
			},
		});
	}

	Ok(())
}
