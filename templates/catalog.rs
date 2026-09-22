impl Translations {
	/// Language represented by this complete, immutable snapshot.
	pub fn locale(&self) -> Locale {
		self.locale
	}

	/// Load every embedded module for one compiled language.
	/// This parses a fresh snapshot without filesystem access; clone an existing
	/// snapshot to share parsed bundles instead.
	pub fn embedded(locale: Locale) -> Self {
		Self::__embedded(locale)
	}

	/// Validate and load a complete language, preserving each module namespace.
	///
	/// Input order does not matter. Missing, duplicate or unexpected paths and
	/// incompatible Fluent schemas reject the entire candidate, not just one module.
	/// Each pair contains a path below the language directory and its complete FTL
	/// source, e.g. `("presentation/hud.ftl", source)`.
	/// These are logical module paths, not filesystem locations or asset-source URLs.
	/// The caller may read/decompress an archive or use any other storage, then pass
	/// its UTF-8 strings here. This method performs no I/O and retains no borrows
	/// into the input buffers. Supply one coherent revision for the entire language;
	/// compatibility validation cannot detect a mix of otherwise valid revisions.
	///
	/// # Errors
	/// Returns module-inventory, Fluent key/reference or upstream typed-contract
	/// diagnostics. Failure leaves any previously loaded snapshot untouched.
	pub fn from_modules(
		locale: Locale,
		modules: &[(&str, &str)],
	) -> ::std::result::Result<Self, ::std::string::String> {
		let mut sources = ::std::collections::BTreeMap::new();
		let expected: ::std::collections::BTreeSet<_> = MODULES
			.iter()
			.filter(|(language, _, _)| *language == locale.as_ref())
			.map(|(_, path, _)| *path)
			.collect();

		for &(path, source) in modules {
			if !expected.contains(path) {
				return ::std::result::Result::Err(format!("unexpected module: {path}"));
			}

			if sources.insert(path, source).is_some() {
				return ::std::result::Result::Err(format!("duplicate module: {path}"));
			}
		}

		let missing: ::std::vec::Vec<_> = expected
			.into_iter()
			.filter(|path| !sources.contains_key(path))
			.collect();

		if !missing.is_empty() {
			return ::std::result::Result::Err(format!("missing modules: {}", missing.join(", ")));
		}

		let mut failures = ::std::vec::Vec::new();

		for &(language, path, embedded) in MODULES {
			if language != locale.as_ref() {
				continue;
			}

			if let ::std::result::Result::Err(error) = __validation::validate(sources[path], embedded) {
				failures.push(format!("{path}: {error}"));
			}
		}

		if !failures.is_empty() {
			return ::std::result::Result::Err(failures.join("\n"));
		}

		Self::__external(locale, &sources)
	}
}
