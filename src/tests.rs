use super::{Settings, generate};
use std::{
	fs,
	path::{Path, PathBuf},
	sync::atomic::{AtomicU64, Ordering},
};
use syn::{Attribute, Item, parse_quote};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

mod documentation;
mod leaf_api;
mod syntax;

struct Fixture(PathBuf);

impl Fixture {
	fn new() -> Self {
		let suffix = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let path = std::env::temp_dir().join(format!(
			"typed-locale-codegen-{}-{suffix}",
			std::process::id()
		));
		fs::create_dir(&path).unwrap();
		Self(path)
	}

	fn write(&self, relative: &str, source: &str) {
		let path = self.0.join(relative);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, source).unwrap();
	}

	fn catalogs(&self) -> Settings {
		self.write("data/strings/localization.toml", configuration());

		for language in ["de", "fr", "pt-BR"] {
			let source = if language == "de" {
				"# $name (String) - User name.\ngreeting = Hallo { $name }\n"
			} else {
				"greeting = Hello { $name }\n"
			};
			self.write(
				&format!("data/strings/languages/{language}/ui/main.ftl"),
				source,
			);
		}

		settings()
	}
}

impl Drop for Fixture {
	fn drop(&mut self) {
		// Only the uniquely created fixture directory belongs to this test.
		let _ = fs::remove_dir_all(&self.0);
	}
}

fn settings() -> Settings {
	Settings {
		asset_root: "data".into(),
		catalog: "strings/localization.toml".into(),
	}
}

fn configuration() -> &'static str {
	r#"
languages-directory = "languages"
source-language = "de"
default-language = "pt-BR"
"#
}

fn metadata() -> &'static str {
	r#"
[package]
name = "example"
[package.metadata.localization]
asset-root = "data"
catalog = "strings/localization.toml"
"#
}

#[test]
fn metadata_has_one_path_source_and_no_language_allowlist() {
	assert_eq!(Settings::from_manifest(metadata()).unwrap(), settings());

	for invalid in [
		metadata().replace("asset-root", "asset-rooot"),
		metadata().replace("asset-root = \"data\"", ""),
		metadata().replace("asset-root = \"data\"", "asset-root = 7"),
		metadata().replace("strings/localization.toml", "../outside.toml"),
		metadata().replace("strings/localization.toml", "/absolute/localization.toml"),
		metadata().replace("strings/localization.toml", "strings/interface.ftl"),
		metadata().replace(
			"strings/localization.toml",
			"strings/localization.toml#label",
		),
		metadata().replace("asset-root = \"data\"", "asset-root = \"../data\""),
	] {
		assert!(Settings::from_manifest(&invalid).is_err(), "{invalid}");
	}
}

#[test]
fn discovers_all_languages_and_nested_modules_in_a_relocated_catalog() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");
	generate(&fixture.0, &output, &settings).unwrap();
	let manifest = fs::read_to_string(output.join("locale_modules.rs")).unwrap();
	let generated = fs::read_to_string(output.join("translations.rs")).unwrap();
	let syntax = syn::parse_file(&manifest).unwrap();
	let modules = syntax
		.items
		.iter()
		.find_map(|item| match item {
			Item::Const(item) if item.ident == "MODULES" => Some(item.expr.as_ref()),
			_ => None,
		})
		.unwrap();
	let syn::Expr::Reference(reference) = modules else {
		panic!("MODULES must be an array reference");
	};
	let syn::Expr::Array(entries) = reference.expr.as_ref() else {
		panic!("MODULES must contain module tuples");
	};

	for language in ["de", "fr", "pt-BR"] {
		let path = fixture
			.0
			.join(format!("data/strings/languages/{language}/ui/main.ftl"));
		let path = path.to_str().unwrap();
		let expected: syn::ExprTuple =
			parse_quote!((#language, "ui/main.ftl", include_str!(#path)));
		assert!(entries.elems.iter().any(|entry| {
			let syn::Expr::Tuple(tuple) = entry else {
				return false;
			};

			tuple.elems.iter().eq(expected.elems.iter())
		}));
		assert!(generated.contains(&format!("\"{language}\" =>")));
	}

	assert!(manifest.contains("ASSET_ROOT: &str = \"data\""));
	assert!(manifest.contains("CATALOG_ASSET_PATH: &str = \"strings/localization.toml\""));
	assert!(manifest.contains("DEFAULT_LANGUAGE: &str = \"pt-BR\""));
	assert!(output.join("modules/ui/main/translations.ftl").is_file());
	assert!(!fixture.0.join("src").exists());

	// Adding another language requires no settings or Rust enum changes.
	fixture.write(
		"data/strings/languages/es/ui/main.ftl",
		"greeting = Hola { $name }\n",
	);
	generate(&fixture.0, &output, &settings).unwrap();
	let regenerated = fs::read_to_string(output.join("translations.rs")).unwrap();
	assert!(regenerated.contains("\"es\" =>"));
}

struct TestExtension;

impl crate::Extension for TestExtension {
	fn filename(&self) -> &str {
		"custom_adapter.rs"
	}

	fn type_attributes(&self) -> Vec<Attribute> {
		vec![parse_quote!(#[allow(dead_code)])]
	}

	fn reserved_names(&self) -> &[&str] {
		&["adapter_reserved", "type"]
	}

	fn root_items(&self, scopes: &[crate::Scope]) -> Vec<Item> {
		assert_eq!(scopes[0].type_path, parse_quote!(Translations));
		assert_eq!(scopes[2].type_path, parse_quote!(ui::Main));
		let accessors: Vec<syn::Ident> = vec![parse_quote!(ui), parse_quote!(main)];
		assert_eq!(scopes[2].accessors, accessors);

		vec![parse_quote! {
			/// custom adapter
			const CUSTOM_ADAPTER: bool = true;
		}]
	}
}

#[test]
fn extension_entrypoint_is_generated_without_changing_raw_outputs() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/custom generated output");
	crate::generate_with(&fixture.0, &output, &settings, &TestExtension).unwrap();
	let entrypoint = fs::read_to_string(output.join("custom_adapter.rs")).unwrap();
	let raw = fs::read(output.join("translations.rs")).unwrap();
	let manifest = fs::read(output.join("locale_modules.rs")).unwrap();

	assert!(entrypoint.contains("// custom adapter"));
	assert!(entrypoint.contains("#[allow(dead_code)]"));
	assert!(entrypoint.contains("include!(\"modules/ui/main/translations.rs\")"));
	assert!(entrypoint.contains("include!(\"locale_modules.rs\")"));
	assert!(!entrypoint.contains("env!(\"OUT_DIR\")"));
	assert!(!String::from_utf8_lossy(&raw).contains("custom adapter"));
	assert!(!fixture.0.join("src").exists());

	fs::write(output.join("custom_adapter.rs"), "stale entrypoint").unwrap();
	crate::generate_with(&fixture.0, &output, &settings, &TestExtension).unwrap();

	assert_eq!(
		fs::read_to_string(output.join("custom_adapter.rs")).unwrap(),
		entrypoint
	);
	assert_eq!(fs::read(output.join("translations.rs")).unwrap(), raw);
	assert_eq!(
		fs::read(output.join("locale_modules.rs")).unwrap(),
		manifest
	);
}

#[test]
fn raw_generation_neither_creates_nor_touches_an_extension_entrypoint() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");
	generate(&fixture.0, &output, &settings).unwrap();
	assert!(!output.join("custom_adapter.rs").exists());
	assert!(output.join("translations.rs").is_file());
	assert!(output.join("modules/ui/main/translations.ftl").is_file());
	assert!(output.join("locale_modules.rs").is_file());

	fs::write(
		output.join("custom_adapter.rs"),
		"previous enabled-feature output",
	)
	.unwrap();
	fs::write(output.join("unrelated.txt"), "keep").unwrap();
	generate(&fixture.0, &output, &settings).unwrap();

	assert_eq!(
		fs::read_to_string(output.join("custom_adapter.rs")).unwrap(),
		"previous enabled-feature output"
	);
	assert_eq!(
		fs::read_to_string(output.join("unrelated.txt")).unwrap(),
		"keep"
	);
}

#[test]
fn extension_reservations_and_output_paths_are_checked_before_writing() {
	struct InvalidOutput<'a>(&'a str);

	impl crate::Extension for InvalidOutput<'_> {
		fn filename(&self) -> &str {
			self.0
		}

		fn root_items(&self, _: &[crate::Scope]) -> Vec<Item> {
			Vec::new()
		}
	}

	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");

	for name in [
		"../outside.rs",
		"/outside.rs",
		"translations.rs",
		"locale_modules.rs",
		"validation.rs",
		"folder\\escape.rs",
		"",
		"text.txt",
	] {
		assert!(
			crate::generate_with(&fixture.0, &output, &settings, &InvalidOutput(name)).is_err(),
			"{name}"
		);
		assert!(!output.exists());
	}

	for language in ["de", "fr", "pt-BR"] {
		fixture.write(
			&format!("data/strings/languages/{language}/adapter-reserved.ftl"),
			"title = Adapter\n",
		);
	}

	let error = crate::generate_with(&fixture.0, &output, &settings, &TestExtension).unwrap_err();
	assert!(error.contains("reserved by the extension"), "{error}");
	assert!(!output.exists());
	generate(&fixture.0, &output, &settings).unwrap();
}

#[test]
fn detects_missing_languages_modules_and_incompatible_contracts() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");
	fixture.write(
		"data/strings/localization.toml",
		&configuration().replace("pt-BR", "ja"),
	);
	assert!(
		generate(&fixture.0, &output, &settings)
			.unwrap_err()
			.contains("ja")
	);
	fixture.write("data/strings/localization.toml", configuration());

	fixture.write(
		"data/strings/languages/fr/ui/main.ftl",
		"greeting = Hello { $wrong }\n",
	);
	assert!(
		generate(&fixture.0, &output, &settings)
			.unwrap_err()
			.contains("fr")
	);

	fixture.write(
		"data/strings/languages/fr/ui/main.ftl",
		"greeting = Hello { $name }\n",
	);
	fixture.write(
		"data/strings/languages/fr/extra.ftl",
		"unexpected = Surprise\n",
	);
	assert!(
		generate(&fixture.0, &output, &settings)
			.unwrap_err()
			.contains("module paths")
	);
}

#[test]
fn renamed_translation_reports_missing_and_extra_paths_without_changing_sources() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let original = fixture.0.join("data/strings/languages/fr/ui/main.ftl");
	let renamed = fixture.0.join("data/strings/languages/fr/ui/main2.ftl");
	fs::rename(&original, &renamed).unwrap();
	let source = fs::read(&renamed).unwrap();
	let output = fixture.0.join("target/generated");
	let error = generate(&fixture.0, &output, &settings).unwrap_err();

	assert!(error.contains("source-language `de`"));
	assert!(error.contains("Missing in `fr`:"));
	assert!(error.contains(&original.to_string_lossy().to_string()));
	assert!(error.contains("Extra in `fr` (no source counterpart):"));
	assert!(error.contains(&renamed.to_string_lossy().to_string()));
	assert!(error.contains("help:"));
	assert!(!original.exists());
	assert_eq!(fs::read(renamed).unwrap(), source);
	assert!(!output.exists());
}

#[test]
fn accepts_duplicate_keys_across_modules_but_rejects_them_inside_one_file() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	for language in ["de", "fr", "pt-BR"] {
		fixture.write(
			&format!("data/strings/languages/{language}/duplicate.ftl"),
			"greeting = Duplicate\n",
		);
	}

	generate(&fixture.0, &fixture.0.join("target/generated"), &settings).unwrap();
	fixture.write(
		"data/strings/languages/de/duplicate.ftl",
		"greeting = One\ngreeting = Two\n",
	);
	let error = generate(&fixture.0, &fixture.0.join("target/generated"), &settings).unwrap_err();
	assert!(error.contains("duplicate Fluent key"), "{error}");
}

#[test]
fn rejects_missing_invalid_and_empty_configuration_files() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");
	let path = fixture.0.join("data/strings/localization.toml");
	fs::remove_file(path).unwrap();
	assert!(
		generate(&fixture.0, &output, &settings)
			.unwrap_err()
			.contains("localization.toml")
	);

	for invalid in ["", "catalog", "languages-directory = 7"] {
		fixture.write("data/strings/localization.toml", invalid);
		assert!(generate(&fixture.0, &output, &settings).is_err());
	}
}

#[test]
fn namespace_collisions_reserved_names_and_file_directory_ambiguity_are_actionable() {
	for (paths, expected) in [
		(vec!["foo-bar.ftl", "foo_bar.ftl"], "collision"),
		(vec!["screen.ftl", "screen/hud.ftl"], "file and a directory"),
		(vec!["Translations.ftl"], "reserved"),
		(vec!["ui/locale.ftl"], "reserved"),
		(vec!["ui/std.ftl"], "reserved"),
		(vec!["ui/self.ftl"], "cannot identify"),
	] {
		let paths: Vec<_> = paths.into_iter().map(String::from).collect();
		let error = super::tree::Node::build(&paths).err().unwrap();
		assert!(error.contains(expected), "{error}");
	}

	assert_eq!(
		super::tree::names("HTTP-status").unwrap(),
		("http_status".into(), "HttpStatus".into())
	);
	assert_eq!(
		super::tree::names("type").unwrap(),
		("r#type".into(), "Type".into())
	);

	let root = super::tree::Node::build(&[String::from("catalog.ftl")]).unwrap();
	assert_eq!(root.ty, "Translations");
	assert_eq!(root.children["catalog"].ty, "Catalog");
	let keyword = super::tree::Node::build(&[String::from("ui/type.ftl")]).unwrap();
	assert!(crate::render::validate_extension(&keyword, Some(&TestExtension)).is_err());
}

#[test]
fn cross_file_references_are_rejected_instead_of_silently_sharing_a_bundle() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();

	for language in ["de", "fr", "pt-BR"] {
		fixture.write(
			&format!("data/strings/languages/{language}/first.ftl"),
			"title = First\n",
		);
		fixture.write(
			&format!("data/strings/languages/{language}/second.ftl"),
			"caption = { title }\n",
		);
	}

	let error = generate(&fixture.0, &fixture.0.join("target/generated"), &settings).unwrap_err();
	assert!(error.contains("second.ftl"), "{error}");
	assert!(error.contains("title"), "{error}");
}

#[test]
fn local_reference_validation_covers_terms_attributes_missing_targets_and_cycles() {
	let valid = crate::schema::schema(
		"-brand = LEGAM\ntitle = { -brand }\n    .hint = Help\ncaption = { title.hint }\n",
	)
	.unwrap();
	super::references::validate(&valid).unwrap();

	for (source, expected) in [
		("caption = { -missing }\n", "-missing"),
		("title = Text\ncaption = { title.absent }\n", "title.absent"),
		("first = { second }\nsecond = { first }\n", "cyclic"),
		("title = { title }\n", "cyclic"),
	] {
		let schema = crate::schema::schema(source).unwrap();
		let error = super::references::validate(&schema).unwrap_err();
		assert!(error.contains(expected), "{error}");
	}
}

#[cfg(unix)]
#[test]
fn configured_language_directory_cannot_bypass_symlink_checks() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	std::os::unix::fs::symlink(
		fixture.0.join("data/strings/languages"),
		fixture.0.join("data/strings/alias"),
	)
	.unwrap();
	fixture.write(
		"data/strings/localization.toml",
		&configuration().replace("\"languages\"", "\"alias\""),
	);
	let error = generate(&fixture.0, &fixture.0.join("target/generated"), &settings).unwrap_err();
	assert!(error.contains("symbolic links"), "{error}");
}

#[test]
fn explicit_settings_reject_invalid_asset_paths_too() {
	let fixture = Fixture::new();
	let mut settings = fixture.catalogs();
	settings.catalog = Path::new("../escape.toml").into();
	assert!(generate(&fixture.0, &fixture.0.join("target/generated"), &settings).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_languages_and_modules_cannot_bypass_discovery_or_recurse_forever() {
	use std::os::unix::fs::symlink;

	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	symlink(
		fixture.0.join("data/strings/languages/fr"),
		fixture.0.join("data/strings/languages/es"),
	)
	.unwrap();
	let error = generate(&fixture.0, &fixture.0.join("target/generated"), &settings).unwrap_err();
	assert!(error.contains("symbolic links"), "{error}");

	let nested = Fixture::new();
	let settings = nested.catalogs();
	symlink(
		nested.0.join("data/strings/languages/de"),
		nested.0.join("data/strings/languages/de/ui/loop"),
	)
	.unwrap();
	let error = generate(&nested.0, &nested.0.join("target/generated"), &settings).unwrap_err();
	assert!(error.contains("symbolic links"), "{error}");
}
