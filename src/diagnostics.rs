//! Build-time diagnostics must work even when the localization catalogs are broken.
use std::{
	collections::BTreeSet,
	io::Write,
	path::{Path, PathBuf},
	process::ExitCode,
};

pub(super) fn build_outcome(result: Result<(), String>, output: &mut impl Write) -> ExitCode {
	let Err(error) = result else {
		return ExitCode::SUCCESS;
	};

	// A broken diagnostic sink must not turn an expected validation failure into a panic.
	let _ = writeln!(output, "error: localization generation failed\n\n{error}");
	ExitCode::FAILURE
}

pub(super) fn module_mismatch(
	language: &str,
	source_language: &str,
	locale_root: &Path,
	reference_root: &Path,
	locale_paths: &[String],
	reference_paths: &[String],
) -> String {
	let locale_root: PathBuf = locale_root.components().collect();
	let reference_root: PathBuf = reference_root.components().collect();
	let actual: BTreeSet<_> = locale_paths.iter().collect();
	let expected: BTreeSet<_> = reference_paths.iter().collect();
	let mut message = format!(
		"Locale `{language}`: .ftl module paths differ from source-language `{source_language}`.\n\n\
		 Source directory: {}\nLocale directory: {}\n",
		reference_root.display(),
		locale_root.display(),
	);

	if !expected.is_subset(&actual) {
		message.push_str(&format!("\nMissing in `{language}`:\n"));

		for path in expected.difference(&actual) {
			message.push_str(&format!("  - {}\n", locale_root.join(path).display()));
		}
	}

	if !actual.is_subset(&expected) {
		message.push_str(&format!(
			"\nExtra in `{language}` (no source counterpart):\n"
		));

		for path in actual.difference(&expected) {
			message.push_str(&format!("  - {}\n", locale_root.join(path).display()));
		}
	}

	message.push_str(
		"\nhelp: Keep identical relative .ftl paths in every language directory.\n\
		 If you renamed a module, apply the same relative path in every language.",
	);
	message
}

#[cfg(test)]
mod tests;
