//! Package metadata locating translation sources for generation.
use crate::paths;
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
	/// Read `[package.metadata.localization]`; no list of languages is required.
	///
	/// # Errors
	/// Rejects invalid TOML, unknown/missing fields and nonportable or absolute catalog paths.
	/// Language settings belong to `localization.toml`, not Cargo metadata.
	///
	/// ```
	/// use fluent_typed_codegen::Settings;
	/// let settings = Settings::from_manifest(r#"
	/// [package.metadata.localization]
	/// catalog = "assets/localizations/localization.toml"
	/// "#)?;
	/// assert_eq!(settings.catalog.to_str(), Some("assets/localizations/localization.toml"));
	/// # Ok::<(), String>(())
	/// ```
	pub fn from_manifest(source: &str) -> Result<Self, String> {
		let document = source
			.parse::<DocumentMut>()
			.map_err(|error| error.to_string())?;
		let table = document
			.get("package")
			.and_then(|item| item.get("metadata"))
			.and_then(|item| item.get("localization"))
			.and_then(|item| item.as_table())
			.ok_or("missing [package.metadata.localization] in Cargo.toml")?;

		for (key, _) in table {
			if key != "catalog" {
				return Err(format!("unknown localization setting: {key}"));
			}
		}

		let value = |key: &str| {
			table
				.get(key)
				.and_then(|item| item.as_str())
				.filter(|value| !value.trim().is_empty())
				.map(str::to_owned)
				.ok_or_else(|| format!("localization.{key} must be a nonempty string"))
		};

		let settings = Self {
			catalog: value("catalog")?.into(),
		};
		settings.validate()?;
		Ok(settings)
	}

	pub(super) fn validate(&self) -> Result<(), String> {
		paths::validate_catalog(&self.catalog)?;

		if self
			.catalog
			.extension()
			.is_none_or(|extension| extension != "toml")
		{
			return Err("localization.catalog must name a TOML configuration file".into());
		}

		Ok(())
	}

	pub(super) fn catalog_path(&self, package: &Path) -> PathBuf {
		package.join(&self.catalog)
	}
}
