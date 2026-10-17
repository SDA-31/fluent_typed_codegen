use super::{Fixture, configuration, settings};
use crate::generate;
use std::fs;

#[test]
fn omitted_directory_generates_from_locale_folders_beside_the_definition() {
	let fixture = Fixture::new();
	let settings = settings();
	let output = fixture.0.join("target/generated");
	let source = "source-language = 'de'\ndefault-language = 'fr'\n";

	for language in ["de", "fr"] {
		fixture.write(
			&format!("data/strings/{language}/ui/menu.ftl"),
			"title = Settings\n",
		);
	}

	for directory in [
		"",
		"translations-directory = '.'\n",
		"languages-directory = '.'\n",
	] {
		fixture.write(
			"data/strings/localization.toml",
			&format!("{source}{directory}"),
		);
		generate(&fixture.0, &output, &settings).unwrap();
		let metadata = fs::read_to_string(output.join("locale_modules.rs")).unwrap();
		assert!(metadata.contains("pub const LANGUAGES_DIRECTORY: &str = \".\";"));
		assert!(metadata.contains("ui/menu.ftl"));
		let api = fs::read_to_string(output.join("translations.rs")).unwrap();
		assert!(api.contains("pub struct Menu"));
	}
}

#[test]
fn legacy_and_current_nested_directory_keys_generate_the_same_api() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");
	generate(&fixture.0, &output, &settings).unwrap();
	let current = fs::read_to_string(output.join("translations.rs")).unwrap();
	let current_metadata = fs::read_to_string(output.join("locale_modules.rs")).unwrap();

	fixture.write(
		"data/strings/localization.toml",
		&configuration().replace("translations-directory", "languages-directory"),
	);
	generate(&fixture.0, &output, &settings).unwrap();
	assert_eq!(
		fs::read_to_string(output.join("translations.rs")).unwrap(),
		current
	);
	assert_eq!(
		fs::read_to_string(output.join("locale_modules.rs")).unwrap(),
		current_metadata
	);
}
