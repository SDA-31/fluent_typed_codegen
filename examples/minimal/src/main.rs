//! Typed translations in a Rust application: build-time generation and a lightweight macro.
l10n::translations!(pub mod texts);

fn main() {
	let translations: texts::Translations = texts::Locale::En.load();
	let presentation: &texts::Presentation = translations.presentation();
	let hud: &texts::presentation::Hud = presentation.hud();
	println!("{}", hud.msg_title());

	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		println!("{}", locale.load().ui().msg_greeting("Ada"));
	}
}

#[cfg(test)]
mod tests;
