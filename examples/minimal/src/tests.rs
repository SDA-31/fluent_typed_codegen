use crate::{plural_key, texts};
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
		let actual = locale.load().numbers().msg_remaining(selector, text);

		assert_eq!(actual.replace(['\u{2068}', '\u{2069}'], ""), expected);
	}

	let english = texts::Locale::En.load();
	assert_eq!(english.numbers().msg_native(0), "Empty");
	assert_eq!(english.numbers().msg_native(1), "One item");
	assert_eq!(english.numbers().msg_native(2), "Several items");
}

#[deny(unused_imports)]
mod inferred {
	l10n::translations!(mod texts);

	pub(super) fn prompt_prefix() -> String {
		// No named payload import: its generated re-export must not warn.
		texts::Locale::En.load().presentation().hud().prompt().s0
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
		let translations: texts::Translations = locale.load();
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
		let translations = locale.load();
		let prompt: texts::presentation::HudPrompt = translations.presentation().hud().prompt();

		assert!(prompt.s0.starts_with(prefix));
	}

	assert!(inferred::prompt_prefix().starts_with("Press"));
}

#[test]
fn supports_renamed_macro_dependency_nested_visibility_and_attributes() {
	let translated: fixture::texts::Translations = fixture::texts::Locale::Es.load();
	let hud: &fixture::texts::presentation::Hud = translated.presentation().hud();

	assert_eq!(hud.msg_title(), "Estado");
	assert_eq!(fixture::texts::MODULES, texts::MODULES);
	assert_eq!(texts::SOURCE_LANGUAGE, "en");
	assert_eq!(texts::DEFAULT_LANGUAGE, "en");
}

#[test]
fn checked_loading_preserves_the_generated_contract() {
	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		let modules: Vec<_> = texts::MODULES
			.iter()
			.filter(|(language, _, _)| *language == locale.as_ref())
			.map(|(_, path, source)| (*path, *source))
			.collect();
		let external = texts::Translations::from_modules(locale, &modules).unwrap();

		assert_eq!(
			external.ui().msg_greeting("Ada"),
			locale.load().ui().msg_greeting("Ada")
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
