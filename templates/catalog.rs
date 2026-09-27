impl Translations {
	/// Validate and load a complete language from logical path/FTL pairs.
	///
	/// Input order does not matter. No filesystem access is performed and no input
	/// buffers are retained. Supply one coherent revision of all known modules.
	///
	/// # Errors
	/// Rejects missing, duplicate or unknown paths and incompatible Fluent contracts.
	pub fn from_modules(
		locale: Locale,
		modules: &[(&str, &str)],
	) -> ::std::result::Result<Self, LoadError> {
		let sources = Self::__sources(modules, ::std::option::Option::Some(locale))?;
		Self::__external(locale, &sources, true)
	}

	/// Load a complete language without checking the generated message contract.
	///
	/// This is a safe Rust method. UTF-8, Fluent parsing and the complete module
	/// inventory are still required. Missing messages or incompatible arguments can
	/// cause upstream accessors to panic or fail to format correctly later.
	///
	/// # Errors
	/// Rejects missing, duplicate or unknown paths and invalid Fluent syntax.
	pub fn from_modules_unchecked(
		locale: Locale,
		modules: &[(&str, &str)],
	) -> ::std::result::Result<Self, LoadError> {
		let sources = Self::__sources(modules, ::std::option::Option::Some(locale))?;
		Self::__external(locale, &sources, false)
	}

	/// Check the complete module inventory and contracts without retaining a catalog.
	///
	/// Parsing uses temporary allocations. Calling a constructor afterwards parses
	/// again; normal checked loading does not require this separate call.
	///
	/// # Errors
	/// Rejects missing, duplicate or unknown paths and incompatible Fluent contracts.
	pub fn validate_modules(modules: &[(&str, &str)]) -> ::std::result::Result<(), LoadError> {
		let sources = Self::__sources(modules, ::std::option::Option::None)?;
		Self::__validate(&sources)
	}

	/// Load complete snapshots of every compiled language from locale/path/FTL triples.
	///
	/// # Errors
	/// Rejects missing languages and any invalid or incomplete language candidate.
	pub fn load_all(
		modules: &[(Locale, &str, &str)],
	) -> ::std::result::Result<::std::collections::HashMap<Locale, Self>, LoadError> {
		Self::__load_all(modules, true)
	}

	/// Load every compiled language without checking message contracts.
	///
	/// # Errors
	/// Still rejects missing languages, invalid inventories and invalid Fluent syntax.
	pub fn load_all_unchecked(
		modules: &[(Locale, &str, &str)],
	) -> ::std::result::Result<::std::collections::HashMap<Locale, Self>, LoadError> {
		Self::__load_all(modules, false)
	}

	/// Read and validate all modules of one language from an explicit manifest.
	///
	/// File manifests perform synchronous I/O here. Embedded manifests borrow their
	/// static bytes. The resulting catalog owns its parsed resources in both cases.
	///
	/// # Errors
	/// Returns source, UTF-8, inventory or message-contract diagnostics.
	pub fn from_manifest(
		locale: Locale,
		manifest: &LocalizationManifest,
	) -> ::std::result::Result<Self, LoadError> {
		let paths: ::std::vec::Vec<_> = MODULES
			.iter()
			.filter(|(language, _)| *language == locale.as_ref())
			.map(|(_, path)| *path)
			.collect();
		let bytes = manifest.read_modules(locale.as_ref(), &paths)?;
		let mut modules = ::std::vec::Vec::with_capacity(bytes.len());

		for ((path, bytes), expected) in bytes.iter().zip(&paths) {
			let source = ::std::str::from_utf8(bytes).map_err(|error| LoadError::Module {
				locale: ::std::option::Option::Some(locale),
				path: (*expected).into(),
				message: error.to_string(),
			})?;
			modules.push((path.as_str(), source));
		}

		Self::from_modules(locale, &modules)
	}

	fn __sources<'a>(
		modules: &'a [(&'a str, &'a str)],
		locale: ::std::option::Option<Locale>,
	) -> ::std::result::Result<::std::collections::BTreeMap<&'a str, &'a str>, LoadError> {
		let expected: ::std::collections::BTreeSet<_> =
			MODULES.iter().map(|(_, path)| *path).collect();
		let mut sources = ::std::collections::BTreeMap::new();

		for &(path, source) in modules {
			if !expected.contains(path) {
				return ::std::result::Result::Err(LoadError::UnexpectedModule {
					locale,
					path: path.into(),
				});
			}

			if sources.insert(path, source).is_some() {
				return ::std::result::Result::Err(LoadError::DuplicateModule {
					locale,
					path: path.into(),
				});
			}
		}

		let missing: ::std::vec::Vec<_> = expected
			.into_iter()
			.filter(|path| !sources.contains_key(path))
			.map(::std::string::String::from)
			.collect();

		if !missing.is_empty() {
			return ::std::result::Result::Err(LoadError::MissingModules {
				locale,
				paths: missing,
			});
		}

		::std::result::Result::Ok(sources)
	}

	fn __load_all(
		modules: &[(Locale, &str, &str)],
		checked: bool,
	) -> ::std::result::Result<::std::collections::HashMap<Locale, Self>, LoadError> {
		let missing: ::std::vec::Vec<_> = Locale::iter()
			.copied()
			.filter(|locale| !modules.iter().any(|(language, _, _)| language == locale))
			.collect();

		if !missing.is_empty() {
			return ::std::result::Result::Err(LoadError::MissingLocales(missing));
		}

		let mut result = ::std::collections::HashMap::new();

		for &locale in Locale::iter() {
			let pairs: ::std::vec::Vec<_> = modules
				.iter()
				.filter(|(language, _, _)| *language == locale)
				.map(|(_, path, source)| (*path, *source))
				.collect();
			let sources = Self::__sources(&pairs, ::std::option::Option::Some(locale))?;
			result.insert(locale, Self::__external(locale, &sources, checked)?);
		}

		::std::result::Result::Ok(result)
	}
}
