//! Preserve upstream hotfixes through discovery, generation and typed error wrapping.
use super::Fixture;
use crate::{BuildError, generate};

#[test]
fn hidden_directories_do_not_enter_locale_or_module_discovery() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();

	for path in [
		"data/strings/languages/.history/not-a-locale.ftl",
		"data/strings/languages/de/.history/broken.ftl",
		"data/strings/languages/fr/ui/.backup/broken.ftl",
	] {
		fixture.write(path, "invalid = {\n");
	}

	generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap();
}

#[test]
fn upstream_rust_name_collisions_remain_typed_build_errors() {
	for source in [
		"foo-bar = First\nfoo_bar = Second\n",
		"a-b = First\na =\n    .b = Second\n",
		"# $myVar (String) - First.\n# $my_var (String) - Second.\nvalue = { $myVar } { $my_var }\n",
		"# $icon (Element) - Slot.\nstring = { $icon }\n",
	] {
		let fixture = Fixture::new();
		let settings = fixture.catalogs();

		for language in ["de", "fr", "pt-BR"] {
			fixture.write(
				&format!("data/strings/languages/{language}/ui/main.ftl"),
				source,
			);
		}

		let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();
		let BuildError::Upstream { module, source } = error else {
			panic!("{error}");
		};

		assert_eq!(module, "ui/main.ftl");
		assert!(
			matches!(
				source.as_ref(),
				fluent_typed::BuildError::NameCollision { .. }
			),
			"{source}"
		);
		assert!(source.to_string().contains("module.ftl:"), "{source}");
	}
}

#[test]
fn non_language_folder_is_rejected_during_generation() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	fixture.write(
		"data/strings/languages/shared/ui/main.ftl",
		"greeting = Shared { $name }\n",
	);
	let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();
	assert!(matches!(error, BuildError::InvalidLocale { locale, .. } if locale == "shared"));
}

#[cfg(unix)]
#[test]
fn hidden_symlinks_still_fail_before_directory_filtering() {
	for relative in [
		"data/strings/languages/.history",
		"data/strings/languages/de/ui/.history",
	] {
		let fixture = Fixture::new();
		let settings = fixture.catalogs();
		std::os::unix::fs::symlink(
			fixture.0.join("data/strings/languages/de"),
			fixture.0.join(relative),
		)
		.unwrap();
		let error = generate(&fixture.0, &fixture.0.join("generated"), &settings).unwrap_err();
		assert!(matches!(error, BuildError::Symlink { .. }));
	}
}
