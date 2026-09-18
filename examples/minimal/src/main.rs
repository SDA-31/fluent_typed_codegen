//! Typed translations in a Rust application: build-time generation and a lightweight macro.
use icu_decimal::{DecimalFormatter, input::Decimal};
use icu_locale_core::Locale;
use icu_plurals::{PluralCategory, PluralRules};

l10n::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let translations: texts::Translations = texts::Locale::En.load();
	let presentation: &texts::Presentation = translations.presentation();
	let hud: &texts::presentation::Hud = presentation.hud();
	println!("{}", hud.msg_title());

	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		let translations = locale.load();
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
mod tests;
