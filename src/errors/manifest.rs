//! Manifest source and requested-module failures.
use std::{fmt, io, path::PathBuf};

/// A manifest or requested module could not be obtained.
#[derive(Debug)]
#[non_exhaustive]
pub enum ManifestError {
	/// A virtual origin must be read by its host loader.
	RequiresLoader {
		/// Host-owned virtual manifest address.
		origin: PathBuf,
	},
	/// Invalid TOML or recognized configuration field.
	Config {
		/// Original typed configuration failure.
		source: crate::ConfigError,
	},
	/// File reading failed, retaining its path and original error.
	Io {
		/// Requested filesystem path.
		path: PathBuf,
		/// Original I/O error.
		source: io::Error,
	},

	/// The explicit embedded set does not contain a requested module.
	MissingModule {
		/// Requested locale code.
		locale: String,
		/// Requested logical module path.
		path: String,
	},
}

impl fmt::Display for ManifestError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),

			Self::Config { source } => source.fmt(formatter),
			Self::RequiresLoader { .. } => {
				formatter.write_str("virtual manifest origins require their host loader")
			}
			Self::MissingModule { locale, path } => {
				write!(formatter, "missing module: {locale}/{path}")
			}
		}
	}
}

impl std::error::Error for ManifestError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Io { source, .. } => Some(source),
			Self::Config { source } => Some(source),
			_ => None,
		}
	}
}
