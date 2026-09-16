use super::{build_outcome, module_mismatch};
use std::{io, path::Path, process::ExitCode};

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
	assert_eq!(
		message,
		"Locale `pt-BR`: .ftl module paths differ from source-language `de`.\n\n\
		 Source directory: data/translated files/de\n\
		 Locale directory: data/translated files/pt-BR\n\n\
		 Missing in `pt-BR`:\n  - data/translated files/pt-BR/ui/old.ftl\n\n\
		 Extra in `pt-BR` (no source counterpart):\n  - data/translated files/pt-BR/ui/a.ftl\n  - data/translated files/pt-BR/ui/z.ftl\n\n\
		 help: Keep identical relative .ftl paths in every language directory.\n\
		 If you renamed a module, apply the same relative path in every language."
	);
}

#[test]
fn missing_only_does_not_report_extra_files() {
	let message = mismatch(&[], &["ui/main.ftl"]);

	assert!(message.contains("Missing in `pt-BR`:"));
	assert!(message.contains("data/translated files/pt-BR/ui/main.ftl"));
	assert!(!message.contains("Extra in"));
}

#[test]
fn extra_only_does_not_report_missing_files() {
	let message = mismatch(&["ui/main.ftl", "extra.ftl"], &["ui/main.ftl"]);

	assert!(message.contains("Extra in `pt-BR` (no source counterpart):"));
	assert!(message.contains("data/translated files/pt-BR/extra.ftl"));
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
