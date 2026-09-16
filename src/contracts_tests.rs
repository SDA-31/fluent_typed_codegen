use crate::{configuration::CatalogConfig, schema};

#[test]
fn catalog_configuration_is_real_strict_toml() {
	let source = "translations-directory = \"languages\"\nsource-language = \"en\"\ndefault-language = \"ru\"\n";
	let config = CatalogConfig::parse(source).unwrap();
	assert_eq!(config.languages_directory.to_str(), Some("languages"));
	assert_eq!(config.source_language, "en");
	assert_eq!(config.default_language, "ru");
	assert_eq!(
		CatalogConfig::parse(&format!("# comment\n{source}")).unwrap(),
		config
	);

	for invalid in [
		String::new(),
		"not a TOML file".into(),
		source.replace("source-language", "source-langauge"),
		source.replace("\"en\"", "7"),
		source.replace("\"ru\"", "\" \""),
		source.replace("\"languages\"", "\"../outside\""),
		source.replace("\"languages\"", "\"/absolute\""),
		source.replace("\"languages\"", "\"folder#label\""),
		source.replace("\"languages\"", "\"folder:source\""),
		source.replace("\"languages\"", "\"\""),
		source.replace("\"languages\"", "\" \""),
		source.replace("\"languages\"", "7"),
		source.replace("\"languages\"", "[]"),
	] {
		assert!(CatalogConfig::parse(&invalid).is_err(), "{invalid}");
	}
}

#[test]
fn catalog_directory_defaults_to_siblings_and_accepts_a_legacy_alias() {
	let source = "source-language = 'en'\ndefault-language = 'es'\n";
	let implicit = CatalogConfig::parse(source).unwrap();
	assert_eq!(implicit.languages_directory, std::path::Path::new("."));

	for key in ["translations-directory", "languages-directory"] {
		let explicit = CatalogConfig::parse(&format!("{source}{key} = '.'\n")).unwrap();
		assert_eq!(implicit, explicit);

		let nested = CatalogConfig::parse(&format!("{source}{key} = 'translations'\n")).unwrap();
		assert_eq!(
			nested.languages_directory,
			std::path::Path::new("translations")
		);

		for invalid in ["7", "[]", "''", "' '", "'../outside'", "'/absolute'"] {
			assert!(CatalogConfig::parse(&format!("{source}{key} = {invalid}\n")).is_err());
		}
	}

	for alias_value in [".", "elsewhere"] {
		let error = CatalogConfig::parse(&format!(
			"{source}translations-directory = '.'\nlanguages-directory = '{alias_value}'\n"
		))
		.unwrap_err();
		assert!(error.contains("use only translations-directory"), "{error}");
	}
}

#[test]
fn schema_rejects_changed_variables_and_duplicate_messages() {
	let expected = "hello = Hello { $name }\n";
	assert!(schema::validate("hello = Hallo { $name }\n", expected).is_ok());
	assert!(schema::validate("hello = Hallo { $other }\n", expected).is_err());
	assert!(schema::validate("hello = A\nhello = B\n", expected).is_err());
}
