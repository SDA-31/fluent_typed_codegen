//! Portable filesystem input paths for catalog discovery.
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
pub(super) fn validate_relative(path: &Path, field: &str) -> Result<(), String> {
	if path.as_os_str().is_empty()
		|| path.to_str().is_none()
		|| path
			.components()
			.any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
		|| path.to_string_lossy().contains(['\\', '#', ':'])
	{
		return Err(format!(
			"localization.{field} must be a UTF-8 relative path without '..', '\\\\', '#' or ':'"
		));
	}

	Ok(())
}
