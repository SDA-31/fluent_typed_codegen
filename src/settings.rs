//! Project metadata shared by generation and the runtime asset configuration.
use crate::paths;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

/// Portable paths locating a consuming package's assets and catalog configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
	/// Asset directory relative to the consuming Cargo package, e.g. `Resources`.
	pub asset_root: PathBuf,
	/// TOML configuration path relative to `asset_root`, e.g. `Localizations/localization.toml`.
	pub catalog: PathBuf,
}

impl Settings {
	/// Read `[package.metadata.localization]`; no list of languages is required.
	///
	/// # Errors
	/// Rejects invalid TOML, unknown/missing fields and nonportable or escaping paths.
	/// Language settings belong to `localization.toml`, not Cargo metadata.
	///
	/// ```
	/// use fluent_typed_codegen::Settings;
	/// let settings = Settings::from_manifest(r#"
	/// [package.metadata.localization]
	/// asset-root = "Resources"
	/// catalog = "Localizations/localization.toml"
	/// "#)?;
	/// assert_eq!(settings.asset_root.to_str(), Some("Resources"));
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
			if !["asset-root", "catalog"].contains(&key) {
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
			asset_root: value("asset-root")?.into(),
			catalog: value("catalog")?.into(),
		};
		settings.validate()?;
		Ok(settings)
	}

	pub(super) fn validate(&self) -> Result<(), String> {
		paths::validate_relative(&self.asset_root, "asset-root")?;
		paths::validate_relative(&self.catalog, "catalog")?;

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
		package.join(&self.asset_root).join(&self.catalog)
	}
}
