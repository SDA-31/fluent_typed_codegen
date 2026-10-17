//! Build-time localization.toml input; runtime integrations own their transport.
#[cfg(any(feature = "build", feature = "manifest"))]
use crate::paths;
#[cfg(any(feature = "build", feature = "manifest"))]
use crate::{ConfigError, ConfigField, FieldError};
use std::path::PathBuf;
#[cfg(any(feature = "build", feature = "manifest"))]
use toml_edit::{DocumentMut, Table};

/// Contents of `localization.toml`, separate from Cargo's catalog path.
///
/// At build time, languages are discovered as directories, not enumerated here.
/// Changes to compiled languages, modules or typed message contracts require
/// regeneration. Runtime manifests may point to a different translation directory
/// with the same contracts. Compatible text edits can be loaded from files without
/// rebuilding; changing explicitly embedded translations requires a rebuild.
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

#[cfg(any(feature = "build", feature = "manifest"))]
impl CatalogConfig {
	/// Parse TOML, ignoring unknown fields and validating recognized fields.
	///
	/// `translations-directory` is optional and defaults to `.` (locale folders
	/// beside the TOML). The legacy `languages-directory` key is also accepted;
	/// specifying both keys is an error, even when their values match.
	///
	/// # Errors
	/// Returns a diagnostic for invalid TOML, missing or invalid recognized fields,
	/// conflicting directory aliases, or unsafe directory paths. Existence,
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
	/// # Ok::<(), fluent_typed_codegen::ConfigError>(())
	/// ```
	pub fn parse(source: &str) -> Result<Self, ConfigError> {
		let document = source
			.parse::<DocumentMut>()
			.map_err(|source| ConfigError::Toml {
				source: Box::new(source),
			})?;
		let table = document.as_table();
		let settings = Self {
			languages_directory: translations_directory(table)?.into(),
			source_language: string_field(table, ConfigField::SourceLanguage)?,
			default_language: string_field(table, ConfigField::DefaultLanguage)?,
		};

		paths::validate_relative(
			&settings.languages_directory,
			ConfigField::TranslationsDirectory,
		)?;

		Ok(settings)
	}
}

#[cfg(any(feature = "build", feature = "manifest"))]
fn translations_directory(table: &Table) -> Result<String, ConfigError> {
	match (
		table.contains_key("translations-directory"),
		table.contains_key("languages-directory"),
	) {
		(true, true) => Err(ConfigError::ConflictingDirectories),
		(true, false) => string_field(table, ConfigField::TranslationsDirectory),
		(false, true) => string_field(table, ConfigField::LanguagesDirectory),
		(false, false) => Ok(".".into()),
	}
}

#[cfg(any(feature = "build", feature = "manifest"))]
pub(crate) fn string_field(table: &Table, field: ConfigField) -> Result<String, ConfigError> {
	let value = table.get(field.as_ref()).ok_or(ConfigError::InvalidField {
		field,
		reason: FieldError::Missing,
	})?;
	let text = value.as_str().ok_or(ConfigError::InvalidField {
		field,
		reason: FieldError::WrongType,
	})?;

	if text.trim().is_empty() {
		return Err(ConfigError::InvalidField {
			field,
			reason: FieldError::Empty,
		});
	}

	Ok(text.to_owned())
}
