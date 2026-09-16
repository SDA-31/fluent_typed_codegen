//! Build-time localization.toml input; runtime integrations own their transport.
use crate::paths;
use std::path::PathBuf;
use toml_edit::{DocumentMut, Table};

/// Contents of `localization.toml`, separate from Cargo's asset-path settings.
///
/// Languages are discovered as directories, not enumerated in this configuration.
/// Changing these settings requires regeneration. Compatible text edits can be
/// loaded externally without rebuilding; embedded translations require a rebuild.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogConfig {
	/// Resolved `translations-directory`, relative to the configuration file.
	/// Defaults to `.`; `languages-directory` is accepted as a legacy TOML alias.
	pub languages_directory: PathBuf,
	/// Canonical locale code whose messages and annotations define the typed API.
	pub source_language: String,
	/// Canonical startup locale emitted as `DEFAULT_LANGUAGE` metadata for callers.
	/// Does not change generated `Locale::default()`, which selects the source language.
	pub default_language: String,
}

impl CatalogConfig {
	/// Parse TOML, rejecting unknown fields, missing language fields and unsafe paths.
	///
	/// `translations-directory` is optional and defaults to `.` (locale folders
	/// beside the TOML). The legacy `languages-directory` key is also accepted;
	/// specifying both keys is an error, even when their values match.
	///
	/// # Errors
	/// Returns a diagnostic for invalid TOML, fields or directory paths. Existence,
	/// canonical locale spelling and module parity are checked by code generation.
	///
	/// ```
	/// use fluent_typed_codegen::CatalogConfig;
	/// let config = CatalogConfig::parse(r#"
	/// source-language = "en"
	/// default-language = "ru"
	/// "#)?;
	/// assert_eq!(config.default_language, "ru");
	/// assert_eq!(config.languages_directory, std::path::Path::new("."));
	/// # Ok::<(), String>(())
	/// ```
	pub fn parse(source: &str) -> Result<Self, String> {
		let document = source
			.parse::<DocumentMut>()
			.map_err(|error| error.to_string())?;
		let table = document.as_table();
		check_fields(
			table,
			&[
				"translations-directory",
				"languages-directory",
				"source-language",
				"default-language",
			],
		)?;
		let settings = Self {
			languages_directory: translations_directory(table)?.into(),
			source_language: string_field(table, "source-language")?,
			default_language: string_field(table, "default-language")?,
		};
		paths::validate_relative(&settings.languages_directory, "translations-directory")?;
		Ok(settings)
	}
}

fn translations_directory(table: &Table) -> Result<String, String> {
	match (
		table.contains_key("translations-directory"),
		table.contains_key("languages-directory"),
	) {
		(true, true) => {
			Err("use only translations-directory; languages-directory is its legacy alias".into())
		}
		(true, false) => string_field(table, "translations-directory"),
		(false, true) => string_field(table, "languages-directory"),
		(false, false) => Ok(".".into()),
	}
}

fn check_fields(table: &Table, allowed: &[&str]) -> Result<(), String> {
	for (key, _) in table {
		if !allowed.contains(&key) {
			return Err(format!("unknown localization setting: {key}"));
		}
	}

	Ok(())
}

fn string_field(table: &Table, key: &str) -> Result<String, String> {
	table
		.get(key)
		.and_then(|item| item.as_str())
		.filter(|value| !value.trim().is_empty())
		.map(str::to_owned)
		.ok_or_else(|| format!("localization.{key} must be a nonempty string"))
}
