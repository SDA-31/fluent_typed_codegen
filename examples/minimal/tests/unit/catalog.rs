use crate::{load, plural_key, texts};
use icu_decimal::{DecimalFormatter, input::Decimal};
use icu_locale_core::Locale;
use icu_plurals::PluralRules;

#[test]
fn decimal_strings_preserve_visible_precision_and_native_selectors_still_work() {
	for (locale, input, expected) in [
		(texts::Locale::En, "1", "1 item left"),
		(texts::Locale::En, "1.0", "1.0 items left"),
		(texts::Locale::Es, "22", "Quedan 22 elementos"),
		(texts::Locale::Ru, "22", "Осталось 22 предмета"),
		(texts::Locale::Ru, "5", "Осталось 5 предметов"),
	] {
		let language: Locale = locale.as_ref().parse().unwrap();
		let formatter = DecimalFormatter::try_new((&language).into(), Default::default()).unwrap();
		let rules = PluralRules::try_new_cardinal((&language).into()).unwrap();
		let value = input.parse::<Decimal>().unwrap();
		let text = formatter.format_to_string(&value);
		let selector = plural_key(rules.category_for(&value));
		let actual = load(locale)
			.unwrap()
			.numbers()
			.msg_remaining(selector, text);

		assert_eq!(actual.replace(['\u{2068}', '\u{2069}'], ""), expected);
	}

	let english = load(texts::Locale::En).unwrap();
	assert_eq!(english.numbers().msg_native(0), "Empty");
	assert_eq!(english.numbers().msg_native(1), "One item");
	assert_eq!(english.numbers().msg_native(2), "Several items");
}

#[deny(unused_imports)]
mod inferred {
	l10n::translations!(mod texts);

	pub(super) fn prompt_prefix() -> String {
		// No named payload import: its generated re-export must not warn.
		texts::Translations::from_manifest(texts::Locale::En, &texts::embed_manifest!())
			.unwrap()
			.presentation()
			.hud()
			.prompt()
			.s0
	}
}

mod fixture {
	use l10n::translations;

	// Local names must not capture the macro's absolute dependency imports.
	mod fluent_typed {}
	mod fluent_syntax {}
	mod std {}

	translations!(
		/// Nested, restricted module using a renamed macro dependency.
		pub(super) mod texts;
	);

	translations!(
		#[cfg(any())]
		mod texts
	);
}

#[test]
fn loads_every_language_with_named_scopes_and_typed_arguments() {
	for (locale, title, greeting) in [
		(texts::Locale::En, "Status", "Hello, Ada!"),
		(texts::Locale::Es, "Estado", "¡Hola, Ada!"),
		(texts::Locale::Ru, "Статус", "Привет, Ada!"),
	] {
		let translations: texts::Translations = load(locale).unwrap();
		let presentation: &texts::Presentation = translations.presentation();
		let hud: &texts::presentation::Hud = presentation.hud();
		let actual = translations.ui().msg_greeting("Ada");

		assert_eq!(hud.msg_title(), title);
		assert_eq!(actual.replace(['\u{2068}', '\u{2069}'], ""), greeting);
		assert_eq!(translations.r#type().msg_title(), title);
	}
}

#[test]
fn structured_results_are_nameable_without_exposing_leaf_modules() {
	for (locale, prefix) in [
		(texts::Locale::En, "Press"),
		(texts::Locale::Es, "Pulsa"),
		(texts::Locale::Ru, "Нажми"),
	] {
		let translations = load(locale).unwrap();
		let prompt: texts::presentation::HudPrompt = translations.presentation().hud().prompt();

		assert!(prompt.s0.starts_with(prefix));
	}

	assert!(inferred::prompt_prefix().starts_with("Press"));
}

#[test]
fn supports_renamed_macro_dependency_nested_visibility_and_attributes() {
	let translated: fixture::texts::Translations = fixture::texts::Translations::from_manifest(
		fixture::texts::Locale::Es,
		&fixture::texts::embed_manifest!(),
	)
	.unwrap();
	let hud: &fixture::texts::presentation::Hud = translated.presentation().hud();

	assert_eq!(hud.msg_title(), "Estado");
	assert_eq!(fixture::texts::MODULES, texts::MODULES);
	assert_eq!(texts::SOURCE_LANGUAGE, "en");
	assert_eq!(texts::DEFAULT_LANGUAGE, "en");
}

#[test]
fn checked_loading_preserves_the_generated_contract() {
	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		let files = files(locale);
		let modules: Vec<_> = files
			.iter()
			.map(|(path, source)| (path.as_str(), source.as_str()))
			.collect();
		let external = texts::Translations::from_modules(locale, &modules).unwrap();

		assert_eq!(
			external.ui().msg_greeting("Ada"),
			load(locale).unwrap().ui().msg_greeting("Ada")
		);
		assert!(texts::Translations::from_modules(locale, &modules[..1]).is_err());

		let mut invalid = modules;
		let ui = invalid
			.iter_mut()
			.find(|(path, _)| *path == "ui.ftl")
			.unwrap();
		ui.1 = "greeting = Missing contract: { $unknown }";

		assert!(texts::Translations::from_modules(locale, &invalid).is_err());
	}
}

#[test]
fn owned_virtual_modules_load_without_files_and_outlive_their_input_buffers() {
	// A decoded archive/index can supply these same owned strings. No filesystem,
	// Bevy source prefix or archive-specific dependency belongs in generated parsing.
	let mut files = files(texts::Locale::En);
	let hud = files.get_mut("presentation/hud.ftl").unwrap();
	*hud = hud.replacen("title = Status", "title = External status", 1);
	let mut current = load(texts::Locale::En).unwrap();
	let borrowed: Vec<_> = files
		.iter()
		.rev()
		.map(|(path, source)| (path.as_str(), source.as_str()))
		.collect();
	let candidate = texts::Translations::from_modules(texts::Locale::En, &borrowed).unwrap();
	assert_eq!(current.presentation().hud().msg_title(), "Status");
	current = candidate;
	drop(borrowed);

	// A malformed next candidate cannot mutate the independently owned snapshot.
	files.insert("ui.ftl".into(), "greeting = { $unknown }".into());
	let borrowed: Vec<_> = files
		.iter()
		.map(|(path, source)| (path.as_str(), source.as_str()))
		.collect();
	assert!(texts::Translations::from_modules(texts::Locale::En, &borrowed).is_err());
	drop(borrowed);
	drop(files);
	assert_eq!(current.presentation().hud().msg_title(), "External status");
	assert!(current.ui().msg_greeting("Ada").contains("Ada"));
}

fn files(locale: texts::Locale) -> std::collections::BTreeMap<String, String> {
	let manifest = texts::embed_manifest!();
	texts::MODULES
		.iter()
		.filter(|(language, _)| *language == locale.as_ref())
		.map(|(_, path)| {
			(
				(*path).into(),
				String::from_utf8(manifest.read(locale.as_ref(), path).unwrap().into_owned())
					.unwrap(),
			)
		})
		.collect()
}

#[test]
fn independent_leaf_loading_is_checked_by_default_and_owns_its_input() {
	let source = b"title = External status\n".to_vec();
	let title = texts::Type::new(texts::Locale::En, &source).unwrap();
	drop(source);
	assert_eq!(title.locale(), texts::Locale::En);
	assert_eq!(texts::Type::PATH, "type.ftl");
	assert_eq!(title.msg_title(), "External status");
	assert!(texts::Type::validate(b"other = valid but incompatible\n").is_err());
	assert!(texts::Type::new(texts::Locale::En, b"other = valid but incompatible\n").is_err());
	assert!(
		texts::Type::new_unchecked(texts::Locale::En, b"other = valid but incompatible\n").is_ok()
	);
	assert!(texts::Type::new_unchecked(texts::Locale::En, &[255]).is_err());
	assert!(texts::Type::new_unchecked(texts::Locale::En, b"title = {\n").is_err());
}

#[test]
fn unchecked_complete_loading_still_requires_exact_inventory() {
	let files = files(texts::Locale::En);
	let mut modules: Vec<_> = files
		.iter()
		.map(|(path, source)| (path.as_str(), source.as_str()))
		.collect();
	assert!(texts::Translations::validate_modules(&modules).is_ok());
	assert!(texts::Translations::from_modules_unchecked(texts::Locale::En, &modules[..1]).is_err());
	modules.push(modules[0]);
	assert!(matches!(
		texts::Translations::from_modules_unchecked(texts::Locale::En, &modules),
		Err(texts::LoadError::DuplicateModule {
			locale: Some(texts::Locale::En),
			..
		})
	));
	modules.pop();
	modules.push(("unknown.ftl", "title = Unknown\n"));
	assert!(matches!(
		texts::Translations::from_modules_unchecked(texts::Locale::En, &modules),
		Err(texts::LoadError::UnexpectedModule {
			locale: Some(texts::Locale::En),
			..
		})
	));
}

#[test]
fn all_language_loading_requires_complete_inputs_and_preserves_locales() {
	let owned: Vec<_> = texts::Locale::iter()
		.map(|&locale| (locale, files(locale)))
		.collect();
	let modules: Vec<_> = owned
		.iter()
		.flat_map(|(locale, files)| {
			files
				.iter()
				.map(move |(path, source)| (*locale, path.as_str(), source.as_str()))
		})
		.collect();
	let all = texts::Translations::load_all(&modules).unwrap();
	let unchecked = texts::Translations::load_all_unchecked(&modules).unwrap();
	assert_eq!(all.len(), texts::Locale::iter().len());
	assert_eq!(unchecked.len(), all.len());

	for &locale in texts::Locale::iter() {
		assert_eq!(all[&locale].locale(), locale);
	}

	let incomplete: Vec<_> = modules
		.iter()
		.copied()
		.filter(|(locale, _, _)| *locale != texts::Locale::Es)
		.collect();
	assert!(matches!(
		texts::Translations::load_all(&incomplete),
		Err(texts::LoadError::MissingLocales(_))
	));
}

#[test]
fn inventory_errors_retain_the_failed_locale_in_all_language_loading() {
	let owned: Vec<_> = texts::Locale::iter()
		.map(|&locale| (locale, files(locale)))
		.collect();
	let modules: Vec<_> = owned
		.iter()
		.flat_map(|(locale, files)| {
			files
				.iter()
				.map(move |(path, source)| (*locale, path.as_str(), source.as_str()))
		})
		.collect();
	let target = texts::Locale::Es;
	let incomplete: Vec<_> = modules
		.iter()
		.copied()
		.filter(|(locale, path, _)| !(*locale == target && *path == "type.ftl"))
		.collect();
	let mut duplicate = modules.clone();
	duplicate.push(
		*modules
			.iter()
			.find(|(locale, path, _)| *locale == target && *path == "type.ftl")
			.unwrap(),
	);
	let mut unexpected = modules.clone();
	unexpected.push((target, "unknown.ftl", "title = Unknown\n"));

	for load in [
		texts::Translations::load_all,
		texts::Translations::load_all_unchecked,
	] {
		let error = load(&incomplete).err().unwrap();
		assert_eq!(error.to_string(), "es: missing modules: type.ftl");
		assert!(
			matches!(error, texts::LoadError::MissingModules { locale: Some(locale), paths } if locale == target && paths == ["type.ftl"])
		);
		let error = load(&duplicate).err().unwrap();
		assert_eq!(error.to_string(), "es: duplicate module: type.ftl");
		assert!(
			matches!(error, texts::LoadError::DuplicateModule { locale: Some(locale), path } if locale == target && path == "type.ftl")
		);
		let error = load(&unexpected).err().unwrap();
		assert_eq!(error.to_string(), "es: unexpected module: unknown.ftl");
		assert!(
			matches!(error, texts::LoadError::UnexpectedModule { locale: Some(locale), path } if locale == target && path == "unknown.ftl")
		);
	}

	let for_target = |entries: &[(texts::Locale, &str, &str)]| {
		let pairs: Vec<_> = entries
			.iter()
			.filter(|(locale, _, _)| *locale == target)
			.map(|(_, path, source)| (*path, *source))
			.collect();
		texts::Translations::validate_modules(&pairs).unwrap_err()
	};

	assert!(matches!(
		for_target(&incomplete),
		texts::LoadError::MissingModules { locale: None, .. }
	));
	assert!(matches!(
		for_target(&duplicate),
		texts::LoadError::DuplicateModule { locale: None, .. }
	));
	assert!(matches!(
		for_target(&unexpected),
		texts::LoadError::UnexpectedModule { locale: None, .. }
	));
}

#[test]
fn assembled_groups_share_ready_leaf_resources_and_reject_mixed_locales() {
	let english = load(texts::Locale::En).unwrap();
	let hud = english.presentation().hud().clone();
	let group = texts::Presentation::__from_parts(texts::Locale::En, hud.clone()).unwrap();
	assert!(std::ptr::eq(&**group.hud(), &*hud));
	assert!(texts::Presentation::__from_parts(texts::Locale::Es, hud).is_err());
}
