//! Error values must remain useful without parsing their displayed diagnostics.
use super::Fixture;
use crate::{
	BuildError, CatalogConfig, ConfigError, ConfigField, FieldError, IoOperation, PathError,
	SchemaError, Settings, generate,
};
use std::{error::Error, fs, io, path::Path};

#[test]
fn known_fields_report_missing_wrong_type_and_empty_separately() {
	for (value, expected) in [
		("", FieldError::Missing),
		("catalog = 7", FieldError::WrongType),
		("catalog = ' '", FieldError::Empty),
	] {
		let input = format!("[package.metadata.localization]\n{value}");
		let error = Settings::from_manifest(&input).unwrap_err();

		assert!(
			matches!(error, ConfigError::InvalidField { field: ConfigField::Catalog, reason } if reason == expected)
		);
	}

	assert!(matches!(
		Settings::from_manifest("[package]\nname='example'"),
		Err(ConfigError::MissingLocalizationTable)
	));
	let config = "source-language='en'\ndefault-language='en'\n";
	assert!(CatalogConfig::parse(&format!("{config}unknown={{ value=7 }}")).is_ok());
	assert!(matches!(
		CatalogConfig::parse(&format!(
			"{config}translations-directory='.'\nlanguages-directory='.'"
		)),
		Err(ConfigError::ConflictingDirectories)
	));
}

#[test]
fn path_failure_retains_field_input_and_reason() {
	for (catalog, expected) in [
		("", PathError::Empty),
		("texts.ftl", PathError::ExpectedToml),
		("texts/catalog:bad.toml", PathError::ForbiddenCharacter(':')),
	] {
		let settings = Settings {
			catalog: catalog.into(),
		};
		let error = settings.validate().unwrap_err();

		assert!(
			matches!(error, ConfigError::InvalidPath { field: ConfigField::Catalog, path, reason } if path == Path::new(catalog) && reason == expected)
		);
	}

	assert!(
		Settings {
			catalog: "../shared/localization#preview.toml".into()
		}
		.validate()
		.is_ok()
	);
	assert!(matches!(
		CatalogConfig::parse(
			"source-language='en'\ndefault-language='en'\ntranslations-directory='../outside'"
		),
		Err(ConfigError::InvalidPath {
			field: ConfigField::TranslationsDirectory,
			reason: PathError::ParentTraversal,
			..
		})
	));
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_are_retained_without_lossy_conversion() {
	use std::{ffi::OsString, os::unix::ffi::OsStringExt, path::PathBuf};

	let path = PathBuf::from(OsString::from_vec(b"catalog-\xff.toml".to_vec()));
	let settings = Settings {
		catalog: path.clone(),
	};
	let error = settings.validate().unwrap_err();

	assert!(
		matches!(error, ConfigError::InvalidPath { path: original, reason: PathError::NonUtf8, .. } if original == path)
	);
}

#[test]
fn configuration_parser_error_remains_in_the_source_chain() {
	let error = CatalogConfig::parse("source-language=[").unwrap_err();
	let source = error.source().unwrap();

	assert!(matches!(error, ConfigError::Toml { .. }));
	assert!(source.downcast_ref::<toml_edit::TomlError>().is_some());
	#[cfg(feature = "manifest")]
	{
		let error =
			crate::LocalizationManifest::parse("source-language=[", "catalog.toml").unwrap_err();
		let config = error
			.source()
			.unwrap()
			.downcast_ref::<ConfigError>()
			.unwrap();

		assert!(
			config
				.source()
				.unwrap()
				.downcast_ref::<toml_edit::TomlError>()
				.is_some()
		);
		assert!(matches!(error, crate::ManifestError::Config { .. }));
	}
}

#[test]
fn missing_input_reports_operation_path_and_original_io_error() {
	let fixture = Fixture::new();
	let settings = Settings {
		catalog: "missing.toml".into(),
	};
	let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();
	let source = error.source().unwrap().downcast_ref::<io::Error>().unwrap();

	assert_eq!(source.kind(), io::ErrorKind::NotFound);
	assert!(
		matches!(error, BuildError::Io { operation: IoOperation::Read, path, source } if path == fixture.0.join("missing.toml") && source.kind() == io::ErrorKind::NotFound)
	);
}

#[test]
fn build_configuration_failure_has_file_context_and_typed_cause() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let path = fixture.0.join("data/strings/localization.toml");
	fs::write(&path, "source-language=7\ndefault-language='fr'").unwrap();
	let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();

	assert!(
		matches!(error, BuildError::Config { path: Some(actual), source: ConfigError::InvalidField { field: ConfigField::SourceLanguage, reason: FieldError::WrongType } } if actual == path)
	);
}

#[test]
fn module_inventory_difference_exposes_sorted_logical_paths() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	fs::remove_file(fixture.0.join("data/strings/languages/fr/ui/main.ftl")).unwrap();
	fixture.write(
		"data/strings/languages/fr/ui/z.ftl",
		"greeting = Z { $name }\n",
	);
	fixture.write(
		"data/strings/languages/fr/ui/a.ftl",
		"greeting = A { $name }\n",
	);
	let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();
	let BuildError::ModuleMismatch(details) = error else {
		panic!("{error}");
	};

	assert_eq!(details.locale, "fr");
	assert_eq!(details.source_language, "de");
	assert_eq!(details.missing, ["ui/main.ftl"]);
	assert_eq!(details.extra, ["ui/a.ftl", "ui/z.ftl"]);
}

#[test]
fn fluent_failure_retains_locale_file_and_parser_ranges() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let path = fixture.0.join("data/strings/languages/fr/ui/main.ftl");
	fs::write(&path, "greeting = {\n").unwrap();
	let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();
	let BuildError::Schema {
		path: actual,
		locale,
		source: SchemaError::Parse(errors),
	} = error
	else {
		panic!("{error}");
	};

	assert_eq!(actual, path);
	assert_eq!(locale, "fr");
	assert!(!errors.is_empty());
	assert!(errors.iter().all(|error| error.pos.start <= error.pos.end));
}

#[test]
fn upstream_typed_argument_failure_remains_available_to_callers() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	fixture.write(
		"data/strings/languages/de/ui/main.ftl",
		"greeting = Hallo { $name }\n",
	);
	let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();
	let source = error.source().unwrap();

	assert!(source.downcast_ref::<fluent_typed::BuildError>().is_some());
	assert!(matches!(error, BuildError::Upstream { module, .. } if module == "ui/main.ftl"));
}
