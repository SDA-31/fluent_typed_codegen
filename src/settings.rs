//! Package metadata locating translation sources for generation.
use crate::{ConfigError, ConfigField, PathError, configuration::string_field, paths};
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

/// A package-relative filesystem path locating the catalog configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
	/// TOML path relative to the consuming Cargo package; `..` can locate shared sources.
	/// For example, `assets/localizations/localization.toml`.
	pub catalog: PathBuf,
}

impl Settings {
	/// Read `[package.metadata.localization]`; unknown fields are ignored.
	/// No list of languages is required.
	///
	/// # Errors
	/// Rejects invalid TOML, missing or invalid `catalog`, and nonportable or absolute paths.
	/// Language settings belong to `localization.toml`, not Cargo metadata.
	///
	/// ```
	/// use fluent_typed_codegen::Settings;
	/// let settings = Settings::from_manifest(r#"
	/// [package.metadata.localization]
	/// catalog = "assets/localizations/localization.toml"
	/// "#)?;
	/// assert_eq!(settings.catalog.to_str(), Some("assets/localizations/localization.toml"));
	/// # Ok::<(), fluent_typed_codegen::ConfigError>(())
	/// ```
	pub fn from_manifest(source: &str) -> Result<Self, ConfigError> {
		let document = source
			.parse::<DocumentMut>()
			.map_err(|source| ConfigError::Toml {
				source: Box::new(source),
			})?;
		let table = document
			.get("package")
			.and_then(|item| item.get("metadata"))
			.and_then(|item| item.get("localization"))
			.and_then(|item| item.as_table())
			.ok_or(ConfigError::MissingLocalizationTable)?;

		let settings = Self {
			catalog: string_field(table, ConfigField::Catalog)?.into(),
		};

		settings.validate()?;

		Ok(settings)
	}

	pub(super) fn validate(&self) -> Result<(), ConfigError> {
		paths::validate_catalog(&self.catalog)?;

		if self
			.catalog
			.extension()
			.is_none_or(|extension| extension != "toml")
		{
			return Err(ConfigError::InvalidPath {
				field: ConfigField::Catalog,
				path: self.catalog.clone(),
				reason: PathError::ExpectedToml,
			});
		}

		Ok(())
	}

	pub(super) fn catalog_path(&self, package: &Path) -> PathBuf {
		package.join(&self.catalog)
	}
}
