//! Structured configuration failures shared by build and runtime manifest parsing.
use std::{error::Error, fmt, path::PathBuf};

/// A localization setting or requested source address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfigField {
	/// Cargo metadata path to the TOML manifest.
	Catalog,
	/// Language defining generated message contracts.
	SourceLanguage,
	/// Initial language selected by the application.
	DefaultLanguage,
	/// Directory containing language folders.
	TranslationsDirectory,
	/// Legacy spelling of the translations directory.
	LanguagesDirectory,
	/// Requested locale path component.
	Locale,
	/// Requested logical FTL module path.
	Module,
}

impl AsRef<str> for ConfigField {
	fn as_ref(&self) -> &str {
		match self {
			Self::Catalog => "catalog",
			Self::SourceLanguage => "source-language",
			Self::DefaultLanguage => "default-language",
			Self::TranslationsDirectory => "translations-directory",
			Self::LanguagesDirectory => "languages-directory",
			Self::Locale => "locale",
			Self::Module => "module",
		}
	}
}

impl fmt::Display for ConfigField {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_ref())
	}
}

/// Why a recognized field cannot be used as a string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldError {
	/// The required field is absent.
	Missing,
	/// The supplied TOML value is not a string.
	WrongType,
	/// The supplied string is empty or whitespace only.
	Empty,
}

impl fmt::Display for FieldError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Missing => "field is missing",
			Self::WrongType => "expected a string",
			Self::Empty => "string is empty or whitespace only",
		})
	}
}

/// Why a filesystem input path is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathError {
	/// The path is empty.
	Empty,
	/// The path cannot be represented by generated UTF-8 strings.
	NonUtf8,
	/// An absolute path or platform prefix was supplied.
	Absolute,
	/// Parent traversal is not allowed for this field.
	ParentTraversal,
	/// A character is not portable for this field.
	ForbiddenCharacter(char),
	/// The catalog filename does not end in `.toml`.
	ExpectedToml,
	/// A locale address must contain exactly one normal path component.
	ExpectedLocaleComponent,
	/// A logical module address must end in `.ftl`.
	ExpectedFtl,
}

impl fmt::Display for PathError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Empty => f.write_str("path is empty"),
			Self::NonUtf8 => f.write_str("path must be UTF-8"),
			Self::Absolute => f.write_str("path must be relative"),
			Self::ParentTraversal => f.write_str("parent components ('..') are not allowed"),
			Self::ForbiddenCharacter(c) => write!(f, "character {c:?} is not allowed"),
			Self::ExpectedToml => f.write_str("catalog must name a TOML configuration file"),
			Self::ExpectedLocaleComponent => f.write_str("expected one locale component"),
			Self::ExpectedFtl => f.write_str("expected a logical .ftl path"),
		}
	}
}

/// Invalid TOML, a recognized setting, or a configuration path.
#[derive(Debug)]
#[non_exhaustive]
pub enum ConfigError {
	/// TOML parsing failed, retaining the parser's diagnostic and span.
	#[cfg(any(feature = "build", feature = "manifest"))]
	Toml {
		/// Original TOML parser error.
		source: Box<toml_edit::TomlError>,
	},
	/// Cargo metadata has no localization table.
	MissingLocalizationTable,
	/// A known field is absent, empty, or has the wrong TOML type.
	InvalidField {
		/// Recognized field.
		field: ConfigField,
		/// Machine-readable failure reason.
		reason: FieldError,
	},
	/// A known field contains an invalid path.
	InvalidPath {
		/// Recognized field.
		field: ConfigField,
		/// Original path, including non-UTF-8 inputs.
		path: PathBuf,
		/// Machine-readable failure reason.
		reason: PathError,
	},
	/// Both the current translations-directory key and its legacy alias occur.
	ConflictingDirectories,
}

impl fmt::Display for ConfigError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			#[cfg(any(feature = "build", feature = "manifest"))]
			Self::Toml { source } => source.fmt(f),
			Self::MissingLocalizationTable => {
				f.write_str("missing [package.metadata.localization] in Cargo.toml")
			}
			Self::InvalidField { field, reason } => write!(
				f,
				"localization.{field} must be a nonempty string ({reason})"
			),
			Self::InvalidPath {
				field,
				path,
				reason,
			} => write!(f, "localization.{field}: {}: {reason}", path.display()),
			Self::ConflictingDirectories => f.write_str(
				"use only translations-directory; languages-directory is its legacy alias",
			),
		}
	}
}

impl Error for ConfigError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			#[cfg(any(feature = "build", feature = "manifest"))]
			Self::Toml { source } => Some(source.as_ref()),
			_ => None,
		}
	}
}
