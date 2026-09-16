use crate::{configuration::CatalogConfig, schema};

#[test]
fn catalog_configuration_is_real_strict_toml() {
	let source = "languages-directory = \"languages\"\nsource-language = \"en\"\ndefault-language = \"ru\"\n";
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
	] {
		assert!(CatalogConfig::parse(&invalid).is_err(), "{invalid}");
	}
}

#[test]
fn schema_rejects_changed_variables_and_duplicate_messages() {
	let expected = "hello = Hello { $name }\n";
	assert!(schema::validate("hello = Hallo { $name }\n", expected).is_ok());
	assert!(schema::validate("hello = Hallo { $other }\n", expected).is_err());
	assert!(schema::validate("hello = A\nhello = B\n", expected).is_err());
}
