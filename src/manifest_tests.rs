use crate::{CatalogConfig, LocalizationManifest, ManifestError};
use std::{borrow::Cow, path::Path};

#[test]
fn embedded_lookup_preserves_entry_and_request_order_without_copying_payloads() {
	const FIRST: &[u8] = b"title = First\n";
	let manifest = LocalizationManifest::__embedded((
		"en",
		"en",
		"translations",
		&[
			("fr", "z.ftl", b"title = French\n"),
			("en", "z.ftl", FIRST),
			("en", "a.ftl", b"title = A\n"),
			("en", "z.ftl", b"title = Duplicate\n"),
		],
	));
	let cloned = manifest.clone();
	let modules = cloned
		.read_modules("en", &["z.ftl", "a.ftl", "z.ftl"])
		.unwrap();
	assert_eq!(
		modules
			.iter()
			.map(|(path, _)| path.as_str())
			.collect::<Vec<_>>(),
		["z.ftl", "a.ftl", "z.ftl"]
	);
	assert!(std::ptr::eq(modules[0].1.as_ref(), FIRST));
	assert!(std::ptr::eq(
		manifest.read("en", "z.ftl").unwrap().as_ref(),
		FIRST
	));
	assert_eq!(
		manifest.read("fr", "z.ftl").unwrap().as_ref(),
		b"title = French\n"
	);
	assert_eq!(manifest.embedded_modules().unwrap()[0].0, "fr");
	assert_eq!(manifest.embedded_modules().unwrap().len(), 4);
}

#[test]
fn embedded_contract_borrows_only_explicit_bytes() {
	let manifest = LocalizationManifest::__embedded((
		"en",
		"en",
		"translations",
		&[("en", "ui/menu.ftl", b"title = Menu\n")],
	));
	assert!(manifest.file_path().is_none());
	assert_eq!(manifest.config().source_language, "en");
	assert_eq!(manifest.embedded_modules().unwrap().len(), 1);
	assert!(matches!(
		manifest.read("en", "ui/menu.ftl").unwrap(),
		Cow::Borrowed(_)
	));
	assert!(matches!(
		manifest.read("fr", "ui/menu.ftl"),
		Err(ManifestError::MissingModule { .. })
	));
	assert!(matches!(
		manifest.read("en", "absent.ftl"),
		Err(ManifestError::MissingModule { .. })
	));
}

#[test]
fn logical_requests_and_manual_configuration_cannot_escape_their_root() {
	let config = CatalogConfig {
		languages_directory: "translations".into(),
		source_language: "en".into(),
		default_language: "en".into(),
	};
	let manifest = LocalizationManifest::from_config(config, "missing/catalog.toml");

	for (locale, path) in [
		("../en", "ui.ftl"),
		("en/other", "ui.ftl"),
		("en", "../ui.ftl"),
		("en", "/ui.ftl"),
		("en", "archive://ui.ftl"),
		("en", "ui.txt"),
	] {
		assert!(matches!(
			manifest.read(locale, path),
			Err(ManifestError::Invalid(_))
		));
	}

	let mut config = manifest.config().clone();
	config.languages_directory = "../outside".into();
	let manifest = LocalizationManifest::from_config(config, "missing/catalog.toml");
	assert!(matches!(
		manifest.read("en", "ui.ftl"),
		Err(ManifestError::Invalid(_))
	));
	assert_eq!(
		manifest.file_path(),
		Some(Path::new("missing/catalog.toml"))
	);
}

#[cfg(feature = "manifest")]
#[test]
fn parsing_contract_performs_no_io_and_preserves_host_origin() {
	let source = "source-language = 'en'\ndefault-language = 'fr'\n";
	let manifest = LocalizationManifest::parse(source, "memory://pack/catalog.toml").unwrap();
	assert_eq!(
		manifest.file_path(),
		Some(Path::new("memory://pack/catalog.toml"))
	);
	assert_eq!(manifest.config().default_language, "fr");
	assert!(manifest.embedded_modules().is_none());
	assert!(matches!(
		manifest.read("en", "ui.ftl"),
		Err(ManifestError::Invalid(_))
	));
}

#[cfg(feature = "manifest")]
#[test]
fn file_factory_reads_only_toml_then_requests_one_module() {
	use std::{
		fs,
		sync::atomic::{AtomicU64, Ordering},
	};
	static NEXT: AtomicU64 = AtomicU64::new(0);
	let directory = std::env::temp_dir().join(format!(
		"fluent-manifest-{}-{}",
		std::process::id(),
		NEXT.fetch_add(1, Ordering::Relaxed)
	));
	fs::create_dir(&directory).unwrap();
	let path = directory.join("catalog.toml");
	fs::write(
		&path,
		"source-language = 'en'\ndefault-language = 'en'\ntranslations-directory = 'texts'\n",
	)
	.unwrap();
	let manifest = LocalizationManifest::from_file(&path).unwrap();
	assert!(matches!(
		manifest.read("en", "ui.ftl"),
		Err(ManifestError::Io { .. })
	));
	fs::create_dir_all(directory.join("texts/en")).unwrap();
	fs::write(directory.join("texts/en/ui.ftl"), b"title = Ready\n").unwrap();
	fs::write(directory.join("texts/en/broken.ftl"), [255]).unwrap();
	assert_eq!(
		manifest.read("en", "ui.ftl").unwrap().as_ref(),
		b"title = Ready\n"
	);
	assert_eq!(manifest.read_modules("en", &["ui.ftl"]).unwrap().len(), 1);
	fs::remove_dir_all(directory).unwrap();
}
