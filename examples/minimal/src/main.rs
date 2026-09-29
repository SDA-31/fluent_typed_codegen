//! Typed translations in a Rust application: build-time generation and a lightweight macro.
use icu_decimal::{DecimalFormatter, input::Decimal};
use icu_locale_core::Locale;
use icu_plurals::{PluralCategory, PluralRules};

l10n::translations!(pub mod texts);

// Explicitly embed the complete source set. Select a relative leaf or group here
// instead of Translations when the application only needs part of the tree.
texts::embed_manifest! {
    const EMBEDDED = Translations;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let translations: texts::Translations = load(texts::Locale::En)?;
	let presentation: &texts::Presentation = translations.presentation();
	let hud: &texts::presentation::Hud = presentation.hud();
	println!("{}", hud.msg_title());

	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		let translations = load(locale)?;
		let language: Locale = locale.as_ref().parse()?;
		let formatter = DecimalFormatter::try_new((&language).into(), Default::default())?;
		let rules = PluralRules::try_new_cardinal((&language).into())?;
		let value = Decimal::from(22);
		let text = formatter.format_to_string(&value);
		let selector = plural_key(rules.category_for(&value));
		println!("{}", translations.ui().msg_greeting("Ada"));
		println!("{}", translations.numbers().msg_remaining(selector, text));
	}

	Ok(())
}

// Embedding is an explicit application choice. Merely generating/accessing the
// schema does not expand this recipe or include its FTL bytes.
fn load(locale: texts::Locale) -> Result<texts::Translations, texts::LoadError> {
	texts::Translations::from_manifest(locale, &EMBEDDED)
}

/// Map ICU's category to a literal Fluent String selector; no plural rules live here.
fn plural_key(category: PluralCategory) -> &'static str {
	match category {
		PluralCategory::Zero => "zero",
		PluralCategory::One => "one",
		PluralCategory::Two => "two",
		PluralCategory::Few => "few",
		PluralCategory::Many => "many",
		PluralCategory::Other => "other",
	}
}

#[cfg(test)]
#[path = "../tests/unit/catalog.rs"]
mod tests;
