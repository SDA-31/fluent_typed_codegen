//! Check build-script tracking through a real, isolated Cargo consumer.
use super::Fixture;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn cargo_tracks_original_modules_and_recovers_after_removal() {
	let fixture = Fixture::new();
	fixture.catalogs();

	for language in ["de", "fr", "pt-BR"] {
		fixture.write(
			&format!("data/strings/languages/{language}/extra.ftl"),
			"title = Extra module\n",
		);
	}

	let generator = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let generator = generator.to_str().unwrap().replace('\\', "/");
	fixture.write(
		"Cargo.toml",
		&format!(
			r#"[package]
name = "codegen-incremental-probe"
version = "0.0.0"
edition = "2024"
exclude = ["data/**"]
[workspace]
[build-dependencies]
fluent_typed_codegen = {{ path = {generator:?} }}
[package.metadata.localization]
asset-root = "data"
catalog = "strings/localization.toml"
"#
		),
	);
	fixture.write(
		"build.rs",
		"fn main() -> std::process::ExitCode { fluent_typed_codegen::build() }\n",
	);
	fixture.write(
		"src/lib.rs",
		"include!(concat!(env!(\"OUT_DIR\"), \"/locale_modules.rs\"));\n",
	);
	check(&fixture, true);
	let output = build_output(&fixture);
	let directives = fs::read_to_string(output.join("output")).unwrap();

	// Directory scanning alone can miss deletion when directory timestamps lag.
	// Every original input must be tracked, not just upstream's staged copies.
	for language in ["de", "fr", "pt-BR"] {
		for module in ["ui/main.ftl", "extra.ftl"] {
			let path = fixture
				.0
				.join(format!("data/strings/languages/{language}/{module}"));
			let directive = format!("cargo::rerun-if-changed={}", path.display());
			assert!(
				directives.lines().any(|line| line == directive),
				"{directive}"
			);
		}
	}

	let missing = fixture.0.join("data/strings/languages/fr/extra.ftl");
	fs::remove_file(&missing).unwrap();
	check(&fixture, false);
	fixture.write("data/strings/languages/fr/extra.ftl", "title = Restored\n");
	check(&fixture, true);

	for language in ["de", "fr", "pt-BR"] {
		fs::remove_file(
			fixture
				.0
				.join(format!("data/strings/languages/{language}/extra.ftl")),
		)
		.unwrap();
	}

	check(&fixture, true);
	let metadata = output.join("out/locale_modules.rs");
	assert!(!fs::read_to_string(&metadata).unwrap().contains("extra.ftl"));
	fixture.write(
		"data/strings/languages/es/ui/main.ftl",
		"greeting = Hola { $name }\n",
	);
	check(&fixture, true);
	assert!(fs::read_to_string(&metadata).unwrap().contains("\"es\""));
	fs::remove_file(fixture.0.join("data/strings/languages/es/ui/main.ftl")).unwrap();
	fs::remove_dir(fixture.0.join("data/strings/languages/es/ui")).unwrap();
	fs::remove_dir(fixture.0.join("data/strings/languages/es")).unwrap();
	check(&fixture, true);
	assert!(!fs::read_to_string(&metadata).unwrap().contains("\"es\""));

	// Upstream may need one settling rebuild after writing its staged inputs.
	// Thereafter unchanged builds must not run generation again.
	check(&fixture, true);
	let modified = fs::metadata(&metadata).unwrap().modified().unwrap();

	for _ in 0..2 {
		check(&fixture, true);
		assert_eq!(
			fs::metadata(&metadata).unwrap().modified().unwrap(),
			modified
		);
	}
}

fn check(fixture: &Fixture, success: bool) {
	// Do not share the outer Cargo target directory: its build holds a lock.
	// Dependencies have already been fetched by the enclosing library build.
	let output = Command::new(env!("CARGO"))
		.current_dir(&fixture.0)
		.args(["check", "--offline", "--quiet", "--target-dir"])
		.arg(fixture.0.join("target"))
		.output()
		.unwrap();
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert_eq!(output.status.success(), success, "{stderr}");

	if !success {
		assert!(stderr.contains("extra.ftl"), "{stderr}");
	}
}

fn build_output(fixture: &Fixture) -> PathBuf {
	fs::read_dir(fixture.0.join("target/debug/build"))
		.unwrap()
		.map(|entry| entry.unwrap().path())
		.find(|path| {
			path.file_name()
				.unwrap()
				.to_string_lossy()
				.starts_with("codegen-incremental-probe-")
				&& path.join("output").is_file()
		})
		.unwrap()
}
