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
	let generated_source = fs::read_to_string(&generated).unwrap();

	for removed_helper in ["__define_embed_scopes", "__embed_scope", "__embed_selected"] {
		assert!(
			!generated_source.contains(removed_helper),
			"{removed_helper}"
		);
	}

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
pub type Interface = SettingsTexts;
"#
	);

	// There is no build.rs in this consumer: only its prepared schema is compiled.
	for (path, _, _) in &originals {
		fs::remove_file(path).unwrap();
	}

	build_and_inspect(
		&fixture,
		&schema,
		r#"
    texts::embed_manifest! {
        #[cfg(any())]
        pub const DISABLED = ui::Options;
    }
"#,
		&[],
	);

	for (path, module, source) in &originals {
		if *module == "ui/options.ftl" {
			fs::write(path, source).unwrap();
		}
	}

	build_and_inspect(
		&fixture,
		&schema,
		r#"
    texts::embed_manifest! {
        /// Only the options leaf, with visibility controlled by its owner.
        pub(crate) const OPTIONS = ui::Options;
        const AGAIN = ui::Options;
    }
    let selected: l10n::LocalizationManifest = OPTIONS;
    assert_eq!(AGAIN.embedded_modules(), selected.embedded_modules());
    assert_eq!(selected.config().default_language, "pt-BR");
    let catalog: Interface = SettingsTexts::from_manifest(texts::Locale::De, &selected).unwrap();
    assert!(catalog.msg_title().contains("FTL_OPTIONS_SENTINEL_813627"));
    let from_const = Interface::from_manifest(texts::Locale::De, &OPTIONS).unwrap();
    assert_eq!(from_const.msg_title(), catalog.msg_title());
    mod consumer {
        pub fn selected() -> l10n::LocalizationManifest {
            super::texts::embed_manifest! { const SOURCE = ui::Options; }
            SOURCE
        }
    }
    assert_eq!(selected.embedded_modules(), consumer::selected().embedded_modules());
    assert_eq!(selected.embedded_modules().unwrap().len(), 3);
    for (locale, path, bytes) in selected.embedded_modules().unwrap() {
        assert_eq!(*path, "ui/options.ftl");
        println!("{locale} {path} {}", std::str::from_utf8(bytes).unwrap());
    }
"#,
		&["ui/options.ftl"],
	);

	// The declaration does not depend on the names visible at its call site. Its
	// owning module can keep the whole generated tree private and export only its API.
	let encapsulated = format!(
		r#"#![deny(warnings)]
mod localization {{
    #[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
    mod texts {{
        use l10n as __fluent_codegen;
        use fluent_typed;
        use fluent_syntax;
        include!({generated:?});
    }}
    pub use texts::ui::Options as Interface;
    pub use texts::Locale;
    pub(crate) use texts::embed_manifest as embed;
    #[allow(dead_code)]
    mod ui {{ pub struct Options; }}
    texts::embed_manifest! {{ pub const HUD = ui::Options; }}
}}
use localization::{{HUD as SOURCE, Interface, Locale}};
localization::embed! {{ const REEXPORTED = ui::Options; }}
"#
	);
	build_and_inspect(
		&fixture,
		&encapsulated,
		r#"
    let source: l10n::LocalizationManifest = SOURCE;
    assert_eq!(source.embedded_modules(), REEXPORTED.embedded_modules());
    let catalog = Interface::from_manifest(Locale::De, &source).unwrap();
    println!("{}", catalog.msg_title());
"#,
		&["ui/options.ftl"],
	);

	for (path, module, source) in &originals {
		if *module == "ui/main.ftl" {
			fs::write(path, source).unwrap();
		}
	}

	// Both ui and ui_extra declare Main: matching a type name alone is insufficient.
	build_and_inspect(
		&fixture,
		&schema,
		r#"
    texts::embed_manifest! { const MAIN = ui::Main; }
    let manifest = MAIN;
    for (_, path, bytes) in manifest.embedded_modules().unwrap() {
        assert_eq!(*path, "ui/main.ftl");
        println!("{}", std::str::from_utf8(bytes).unwrap());
    }
"#,
		&["ui/main.ftl"],
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
    texts::embed_manifest! {
        const GROUP = Ui;
        const NESTED = ui::r#type::Detail;
    }
    let group = GROUP;
    let leaf = NESTED;
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
    let complete = texts::embed_manifest!();
    texts::embed_manifest! { const ALL = Translations; }
    assert_eq!(ALL.embedded_modules(), complete.embedded_modules());
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

	// Invalid selectors must not expand even one include, independently of diagnostics.
	for (path, _, _) in &originals {
		fs::remove_file(path).unwrap();
	}

	// Removed expression selectors must fail before resolving paths or reading files.
	for argument in [
		"module = texts::ui::Options",
		"module = texts::Ui",
		"module = texts::Translations",
		"module =",
		"texts::ui::Options",
		"texts::ui::Options,",
		"::application::texts::ui::Options",
		"self::texts::ui::Options",
		"Interface",
		"\"ui/options.ftl\"",
		"texts::ui::Options::<()>",
	] {
		let expression = format!("texts::embed_manifest!({argument})");
		assert_rejected(
			&fixture,
			&schema,
			&expression,
			"use `embed_manifest!()` for the complete tree or",
		);
	}

	for (selector, diagnostic) in [
		("texts::ui::Options", "unknown relative catalog selector"),
		(
			"crate::texts::ui::Options",
			"unknown relative catalog selector",
		),
		("SettingsTexts", "unknown relative catalog selector"),
		("Interface", "unknown relative catalog selector"),
		("ui::Missing", "unknown relative catalog selector"),
		("std::string::String", "unknown relative catalog selector"),
		(
			"\"ui/options.ftl\"",
			"expected `pub const NAME = relative::Scope;`",
		),
		(
			"ui::Options::<()>",
			"expected `pub const NAME = relative::Scope;`",
		),
		(
			"::ui::Options",
			"expected `pub const NAME = relative::Scope;`",
		),
	] {
		let expression =
			format!("{{ texts::embed_manifest! {{ pub const SELECTED = {selector}; }} SELECTED }}");
		assert_rejected(&fixture, &schema, &expression, diagnostic);
	}

	assert_rejected(
		&fixture,
		&schema,
		"texts::ui::Options!(@manifest)",
		"could not find `Options` in `ui`",
	);
	assert_rejected(
		&fixture,
		&schema,
		"SettingsTexts!(@manifest)",
		"cannot find macro `SettingsTexts`",
	);
}

fn assert_rejected(fixture: &Fixture, schema: &str, expression: &str, diagnostic: &str) {
	fixture.write(
		"src/main.rs",
		&format!("{schema}\nfn main() {{ let _ = {expression}; }}\n"),
	);
	let output = cargo(fixture, "check");
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		!output.status.success(),
		"unexpectedly accepted: {expression}"
	);
	assert!(stderr.contains(diagnostic), "{expression}: {stderr}");
	assert!(!stderr.contains("couldn't read"), "{expression}: {stderr}");
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
