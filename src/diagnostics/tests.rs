use super::{build_outcome, module_mismatch};
use std::{
	io,
	path::{Path, PathBuf},
	process::ExitCode,
};

fn expected_root(language: &str) -> PathBuf {
	// Diagnostics use native filesystem paths, not portable asset identifiers.
	["data", "translated files", language].iter().collect()
}

fn mismatch(actual: &[&str], expected: &[&str]) -> String {
	module_mismatch(
		"pt-BR",
		"de",
		Path::new("data/translated files/./pt-BR"),
		Path::new("data/translated files/./de"),
		&actual.iter().map(|path| (*path).into()).collect::<Vec<_>>(),
		&expected
			.iter()
			.map(|path| (*path).into())
			.collect::<Vec<_>>(),
	)
}

#[test]
fn renamed_nested_modules_report_both_sides_in_stable_order() {
	let message = mismatch(&["ui/z.ftl", "ui/a.ftl"], &["ui/old.ftl"]);
	let source = expected_root("de");
	let locale = expected_root("pt-BR");
	let missing = locale.join("ui/old.ftl");
	let extra_a = locale.join("ui/a.ftl");
	let extra_z = locale.join("ui/z.ftl");

	assert_eq!(
		message,
		format!(
			"Locale `pt-BR`: .ftl module paths differ from source-language `de`.\n\n\
			 Source directory: {}\n\
			 Locale directory: {}\n\n\
			 Missing in `pt-BR`:\n  - {}\n\n\
			 Extra in `pt-BR` (no source counterpart):\n  - {}\n  - {}\n\n\
			 help: Keep identical relative .ftl paths in every language directory.\n\
			 If you renamed a module, apply the same relative path in every language.",
			source.display(),
			locale.display(),
			missing.display(),
			extra_a.display(),
			extra_z.display(),
		)
	);
}

#[test]
fn missing_only_does_not_report_extra_files() {
	let message = mismatch(&[], &["ui/main.ftl"]);
	let missing = expected_root("pt-BR").join("ui/main.ftl");

	assert!(message.contains("Missing in `pt-BR`:"));
	assert!(message.contains(missing.to_str().unwrap()), "{message}");
	assert!(!message.contains("Extra in"));
}

#[test]
fn extra_only_does_not_report_missing_files() {
	let message = mismatch(&["ui/main.ftl", "extra.ftl"], &["ui/main.ftl"]);
	let extra = expected_root("pt-BR").join("extra.ftl");

	assert!(message.contains("Extra in `pt-BR` (no source counterpart):"));
	assert!(message.contains(extra.to_str().unwrap()), "{message}");
	assert!(!message.contains("Missing in"));
}

#[test]
fn build_outcome_preserves_readable_lines_and_failure_status_without_panicking() {
	let mut output = Vec::new();
	assert_eq!(build_outcome(Ok(()), &mut output), ExitCode::SUCCESS);
	assert!(output.is_empty());
	assert_eq!(
		build_outcome(Err("Missing:\n  - ru/cli.ftl".into()), &mut output),
		ExitCode::FAILURE
	);
	assert_eq!(
		String::from_utf8(output).unwrap(),
		"error: localization generation failed\n\nMissing:\n  - ru/cli.ftl\n"
	);
}

#[test]
fn failing_diagnostic_sink_still_returns_failure_without_panicking() {
	struct Broken;

	impl io::Write for Broken {
		fn write(&mut self, _: &[u8]) -> io::Result<usize> {
			Err(io::ErrorKind::BrokenPipe.into())
		}

		fn flush(&mut self) -> io::Result<()> {
			Ok(())
		}
	}

	assert_eq!(
		build_outcome(Err("bad catalogs".into()), &mut Broken),
		ExitCode::FAILURE
	);
}
