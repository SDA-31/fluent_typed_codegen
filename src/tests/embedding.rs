//! Embedding must be selective before optimization and linker dead-code removal.
use super::Fixture;
use crate::generate;
use std::{fs, process::Command};

const PAYLOADS: [(&str, &str); 4] = [
	("ui/main.ftl", "FTL_MAIN_SENTINEL_762519"),
	("ui/options.ftl", "FTL_OPTIONS_SENTINEL_813627"),
	("ui/type/detail.ftl", "FTL_NESTED_SENTINEL_927461"),
	("ui_extra/main.ftl", "FTL_OUTSIDE_SENTINEL_418593"),
];

#[test]
fn typed_embedding_keeps_unselected_payloads_out_of_unoptimized_binaries() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let mut originals = Vec::new();

	for language in ["de", "fr", "pt-BR"] {
		for (module, sentinel) in PAYLOADS {
			let relative = format!("data/strings/languages/{language}/{module}");
			let text = format!("title = {sentinel}\n");
			fixture.write(&relative, &text);
			originals.push((fixture.0.join(relative), module, text));
		}
	}

	let output = fixture.0.join("generated");
	generate(&fixture.0, &output, &settings).unwrap();
	let generated = output.join("translations.rs");
	let generator = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
	fixture.write(
		"Cargo.toml",
		&format!(
			r#"[package]
name = "typed-embedding-probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
l10n = {{ package = "fluent_typed_codegen", path = {generator:?}, default-features = false }}
fluent-typed = {{ version = "0.9.0", default-features = false, features = ["langneg"] }}
fluent-syntax = "0.12"
[profile.dev]
opt-level = 0
lto = false
debug = 2
strip = "none"
"#
		),
	);
	let schema = format!(
		r#"#![deny(warnings)]
#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
mod texts {{
    use l10n as __fluent_codegen;
    use fluent_typed;
    use fluent_syntax;
    include!({generated:?});
}}
pub use texts::ui::Options as SettingsTexts;
"#
	);

	// There is no build.rs in this consumer: only its prepared schema is compiled.
	for (path, _, _) in &originals {
		fs::remove_file(path).unwrap();
	}

	build_and_inspect(&fixture, &schema, "", &[]);

	for (path, module, source) in &originals {
		if *module == "ui/options.ftl" {
			fs::write(path, source).unwrap();
		}
	}

	build_and_inspect(
		&fixture,
		&schema,
		r#"
    let direct = texts::embed_manifest!(module = texts::ui::Options);
    let alias = texts::embed_manifest!(module = SettingsTexts,);
    assert_eq!(direct.embedded_modules(), alias.embedded_modules());
    assert_eq!(direct.embedded_modules().unwrap().len(), 3);
    for (locale, path, bytes) in direct.embedded_modules().unwrap() {
        assert_eq!(*path, "ui/options.ftl");
        println!("{locale} {path} {}", std::str::from_utf8(bytes).unwrap());
    }
"#,
		&["ui/options.ftl"],
	);

	for (path, module, source) in &originals {
		if module.starts_with("ui/") {
			fs::write(path, source).unwrap();
		}
	}

	build_and_inspect(
		&fixture,
		&schema,
		r#"
    use texts::ui::r#type as nested;
    let group = texts::embed_manifest!(module = crate::texts::Ui);
    let leaf = texts::embed_manifest!(module = nested::Detail);
    assert_eq!(leaf.embedded_modules().unwrap().len(), 3);
    assert_eq!(group.embedded_modules().unwrap().len(), 9);
    for (locale, path, bytes) in group.embedded_modules().unwrap() {
        assert!(path.starts_with("ui/"));
        println!("{locale} {path} {}", std::str::from_utf8(bytes).unwrap());
    }
"#,
		&["ui/main.ftl", "ui/options.ftl", "ui/type/detail.ftl"],
	);

	for (path, _, source) in &originals {
		fs::write(path, source).unwrap();
	}

	build_and_inspect(
		&fixture,
		&schema,
		r#"
    let explicit = texts::embed_manifest!(module = texts::Translations);
    let complete = texts::embed_manifest!();
    assert_eq!(explicit.embedded_modules(), complete.embedded_modules());
    assert_eq!(complete.embedded_modules().unwrap().len(), 12);
    for (locale, path, bytes) in complete.embedded_modules().unwrap() {
        println!("{locale} {path} {}", std::str::from_utf8(bytes).unwrap());
    }
"#,
		&[
			"ui/main.ftl",
			"ui/options.ftl",
			"ui/type/detail.ftl",
			"ui_extra/main.ftl",
		],
	);

	for (selector, diagnostic) in [
		("texts::ui::Missing", "Missing"),
		("\"ui/options.ftl\"", "no rules expected"),
	] {
		fixture.write(
			"src/main.rs",
			&format!(
				"{schema}\nfn main() {{ let _ = texts::embed_manifest!(module = {selector}); }}\n"
			),
		);
		let output = cargo(&fixture, "check");
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert!(
			!output.status.success(),
			"selector unexpectedly accepted: {selector}"
		);
		assert!(stderr.contains(diagnostic), "{stderr}");
	}
}

fn build_and_inspect(fixture: &Fixture, schema: &str, body: &str, selected: &[&str]) {
	fixture.write(
		"src/main.rs",
		&format!("{schema}\nfn main() {{ {body} }}\n"),
	);
	let output = cargo(fixture, "build");
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(output.status.success(), "{stderr}");
	assert!(stderr.contains("unoptimized + debuginfo"), "{stderr}");
	let executable = fixture.0.join("target/debug").join(format!(
		"typed-embedding-probe{}",
		std::env::consts::EXE_SUFFIX
	));
	let binary = fs::read(&executable).unwrap();
	let run = Command::new(&executable).output().unwrap();
	assert!(
		run.status.success(),
		"{}",
		String::from_utf8_lossy(&run.stderr)
	);

	for (module, sentinel) in PAYLOADS {
		let expected = selected.contains(&module);
		let contains = |bytes: &[u8]| {
			bytes
				.windows(sentinel.len())
				.any(|window| window == sentinel.as_bytes())
		};
		assert_eq!(contains(&binary), expected, "binary payload: {module}");
		assert_eq!(contains(&run.stdout), expected, "runtime payload: {module}");
	}
}

fn cargo(fixture: &Fixture, operation: &str) -> std::process::Output {
	Command::new(env!("CARGO"))
		.current_dir(&fixture.0)
		.args([operation, "--offline", "--target-dir"])
		.arg(fixture.0.join("target"))
		// Also retain dead code, so absent payloads cannot be credited to the linker.
		.env(
			"CARGO_ENCODED_RUSTFLAGS",
			"-Copt-level=0\x1f-Clto=off\x1f-Clink-dead-code=yes",
		)
		.env("CARGO_PROFILE_DEV_OPT_LEVEL", "0")
		.env("CARGO_PROFILE_DEV_LTO", "false")
		.env("CARGO_PROFILE_DEV_DEBUG", "2")
		.env("CARGO_PROFILE_DEV_STRIP", "none")
		.output()
		.unwrap()
}
