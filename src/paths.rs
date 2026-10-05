//! Portable filesystem input paths for catalog discovery.
use crate::{ConfigError, ConfigField, PathError};
use std::path::{Component, Path};

/// Validate a relative path without filesystem access or normalization.
///
/// `field` identifies the setting in diagnostics. A single `.` is accepted;
/// absolute paths, parent components, backslashes, `:` and `#` are not.
/// Filesystem existence and symlink checks belong to discovery, not this function.
///
/// # Errors
/// Rejects empty/non-UTF-8 paths, parent/root/prefix components, `\\`, `#` and `:`.
///
pub(super) fn validate_relative(path: &Path, field: ConfigField) -> Result<(), ConfigError> {
	validate(path, field, false, true)
}

fn validate(path: &Path, field: ConfigField, parents: bool, hash: bool) -> Result<(), ConfigError> {
	let reason = if path.as_os_str().is_empty() {
		Some(PathError::Empty)
	} else if path.to_str().is_none() {
		Some(PathError::NonUtf8)
	} else if path
		.components()
		.any(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
	{
		Some(PathError::Absolute)
	} else if !parents && path.components().any(|c| c == Component::ParentDir) {
		Some(PathError::ParentTraversal)
	} else {
		path.to_str()
			.unwrap()
			.chars()
			.find(|&c| matches!(c, '\\' | ':') || (hash && c == '#'))
			.map(PathError::ForbiddenCharacter)
	};

	if let Some(reason) = reason {
		return Err(ConfigError::InvalidPath {
			field,
			path: path.into(),
			reason,
		});
	}

	Ok(())
}

/// Validate a package-relative filesystem input, independently of engine asset syntax.
/// Parent components may locate shared sources; `#` is an ordinary filename character.
#[cfg(feature = "build")]
pub(super) fn validate_catalog(path: &Path) -> Result<(), ConfigError> {
	validate(path, ConfigField::Catalog, true, false)
}
