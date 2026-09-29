//! Compile the generated runtime independently of the host build graph and source files.
use super::Fixture;
use crate::generate;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn schema_only_compiles_without_ftl_and_checked_loading_enforces_upstream_contracts() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("generated");
	let contract = "language-name = EMBED_PAYLOAD_SENTINEL_482917\n# $enabled (Bool) - State.\nflag = { $enabled ->\n    [true] Enabled\n   *[false] Disabled\n}\n# $icon (Element) - Slot.\nprompt = Press { $icon } now\n";
	let mut originals = Vec::new();

	for language in ["de", "fr", "pt-BR"] {
		let options = format!("data/strings/languages/{language}/ui/options.ftl");
		fixture.write(&options, contract);

		for module in ["ui/main.ftl", "ui/options.ftl"] {
			let path = fixture
				.0
				.join(format!("data/strings/languages/{language}/{module}"));
			originals.push((path.clone(), fs::read(&path).unwrap()));
		}
	}

	generate(&fixture.0, &output, &settings).unwrap();
	let upstream = fs::read_to_string(output.join("modules/ui/options/translations.rs")).unwrap();
	assert!(!upstream.contains("EMBED_PAYLOAD_SENTINEL_482917"));
	assert!(!upstream.contains("LANG_DATA"));
	assert!(!upstream.contains("include_bytes!"));
	let metadata = fs::read_to_string(output.join("locale_modules.rs")).unwrap();
	assert!(!metadata.contains("EMBED_PAYLOAD_SENTINEL_482917"));

	for (path, _) in &originals {
		fs::remove_file(path).unwrap();
	}

	let generator = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let generator = generator.to_str().unwrap().replace('\\', "/");
	fixture.write(
		"Cargo.toml",
		&format!(
			r#"[package]
name = "codegen-runtime-probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
l10n = {{ package = "fluent_typed_codegen", path = {generator:?}, default-features = false }}
fluent-typed = {{ version = "0.9.0", default-features = false, features = ["langneg"] }}
fluent-syntax = "0.12"
"#
		),
	);
	let generated = output.join("translations.rs");
	let source = format!(
		r#"#![deny(warnings)]
#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
pub mod texts {{
    use l10n as __fluent_codegen;
    use fluent_typed;
    use fluent_syntax;
    include!({generated:?});
}}
"#
	);
	fixture.write("src/lib.rs", &source);
	cargo(&fixture, &["check", "--offline", "--quiet"]);

	for (path, bytes) in &originals {
		if path.ends_with("options.ftl") {
			fs::write(path, bytes).unwrap();
		}
	}

	fixture.write(
		"src/lib.rs",
		&format!("{source}\ntexts::embed_manifest! {{ pub const SELECTED = ui::Options; }}\n"),
	);
	cargo(&fixture, &["check", "--locked", "--offline", "--quiet"]);

	for (path, bytes) in originals {
		fs::write(path, bytes).unwrap();
	}

	fixture.write(
		"src/lib.rs",
		&format!(
			"{source}\n{}",
			r#"
#[test]
fn standalone_validation_preserves_bool_and_element_contracts() {
    let manifest = texts::embed_manifest!();
    let bytes = manifest.read("de", "ui/options.ftl").unwrap();
    let source = std::str::from_utf8(&bytes).unwrap();
    let valid = texts::ui::Options::new(texts::Locale::De, &bytes).unwrap();
    assert_eq!(valid.msg_flag(true), "Enabled");
    let bad_bool = source.replace("[true]", "[maybe]");
    let bad_element = source.replace("Press { $icon } now", "Press { $icon } then { $icon }");

    for bad in [bad_bool, bad_element] {
        assert!(texts::ui::Options::validate(bad.as_bytes()).is_err());
        assert!(texts::ui::Options::new(texts::Locale::De, bad.as_bytes()).is_err());
        assert!(texts::ui::Options::new_unchecked(texts::Locale::De, bad.as_bytes()).is_ok());
    }

    assert_eq!(valid.msg_language_name(), "EMBED_PAYLOAD_SENTINEL_482917");
    let all = texts::Translations::from_manifest(texts::Locale::De, &manifest).unwrap();
    let main = all.ui().main().clone();
    let options = all.ui().options().clone();
    let ui = texts::Ui::__from_parts(texts::Locale::De, main, options).unwrap();
    assert!(std::ptr::eq(&**ui.options(), &**all.ui().options()));
}
"#
		),
	);
	cargo(&fixture, &["test", "--locked", "--offline", "--quiet"]);
}

#[test]
fn generation_features_stay_in_the_host_dependency_graph() {
	let fixture = Fixture::new();
	let generator = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	let generator = generator.to_str().unwrap().replace('\\', "/");
	fixture.write(
		"build.rs",
		"fn main() { let _: fn() -> std::process::ExitCode = l10n::build; }\n",
	);

	for resolver in ["2", "3"] {
		fixture.write("Cargo.toml", &format!(r#"[package]
name = "codegen-context-probe"
version = "0.0.0"
edition = "2024"
[workspace]
resolver = {resolver:?}
[dependencies]
l10n = {{ package = "fluent_typed_codegen", path = {generator:?}, default-features = false }}
[build-dependencies]
l10n = {{ package = "fluent_typed_codegen", path = {generator:?}, default-features = false, features = ["build"] }}
"#));
		fixture.write(
			"src/lib.rs",
			"pub fn contract(value: l10n::LocalizationManifest) -> l10n::LocalizationManifest { value }\n",
		);
		cargo(&fixture, &["check", "--offline", "--quiet"]);
		fixture.write(
			"src/lib.rs",
			"use l10n::build;\npub fn runtime_build() { let _ = build; }\n",
		);
		let output = run_cargo(&fixture, &["check", "--locked", "--offline", "--quiet"]);
		let stderr = String::from_utf8_lossy(&output.stderr);
		assert!(
			!output.status.success(),
			"resolver {resolver} leaked host generation APIs"
		);
		assert!(
			stderr.contains("E0432") && stderr.contains("l10n::build"),
			"{stderr}"
		);
	}
}

fn run_cargo(fixture: &Fixture, args: &[&str]) -> std::process::Output {
	Command::new(env!("CARGO"))
		.current_dir(&fixture.0)
		.args(args)
		.arg("--target-dir")
		.arg(fixture.0.join("target"))
		.output()
		.unwrap()
}

fn cargo(fixture: &Fixture, args: &[&str]) {
	let output = run_cargo(fixture, args);
	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
}
