//! Typed translations in a Rust application: build-time generation and a lightweight macro.
use fluent_typed_decimal::{Decimal, NumberFormatter, PluralRuleType};

l10n::translations!(pub mod texts);

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let translations: texts::Translations = texts::Locale::En.load();
	let presentation: &texts::Presentation = translations.presentation();
	let hud: &texts::presentation::Hud = presentation.hud();
	println!("{}", hud.msg_title());

	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		let translations = locale.load();
		let formatter = NumberFormatter::try_new(&locale.as_ref().parse()?, Default::default())?;
		let number = formatter.localize(&Decimal::from(22), PluralRuleType::Cardinal)?;
		println!("{}", translations.ui().msg_greeting("Ada"));
		println!(
			"{}",
			translations
				.numbers()
				.msg_remaining(number.selector(), number.text())
		);
	}

	Ok(())
}

#[cfg(test)]
mod tests;
