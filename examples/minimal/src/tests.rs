use crate::texts;

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
