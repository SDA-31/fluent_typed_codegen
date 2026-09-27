/// Failure to load a complete language or one of its modules.
#[derive(Debug)]
pub enum LoadError {
	/// A module failed UTF-8, schema or Fluent validation/construction.
	Module {
		/// Language, when construction requested one; absent for standalone validation.
		locale: ::std::option::Option<Locale>,
		/// Logical module path.
		path: ::std::string::String,
		/// Original validation or parser diagnostic.
		message: ::std::string::String,
	},
	/// A path does not belong to the generated schema.
	UnexpectedModule {
		/// Language, or absent for standalone validation.
		locale: ::std::option::Option<Locale>,
		/// Unexpected logical path.
		path: ::std::string::String,
	},
	/// A path was supplied more than once for one language.
	DuplicateModule {
		/// Language, or absent for standalone validation.
		locale: ::std::option::Option<Locale>,
		/// Repeated logical path.
		path: ::std::string::String,
	},
	/// A complete language is missing required modules.
	MissingModules {
		/// Language, or absent for standalone validation.
		locale: ::std::option::Option<Locale>,
		/// Required logical paths that were omitted.
		paths: ::std::vec::Vec<::std::string::String>,
	},
	/// The all-language input omits compiled languages.
	MissingLocales(::std::vec::Vec<Locale>),
	/// Ready child scopes use different languages.
	LocaleMismatch,
	/// Reading a requested module from a manifest failed.
	Manifest(ManifestError),
}

impl ::std::fmt::Display for LoadError {
	fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
		match self {
			Self::Module {
				locale,
				path,
				message,
			} => {
				if let ::std::option::Option::Some(locale) = locale {
					write!(formatter, "{locale}/{path}: {message}")
				} else {
					write!(formatter, "{path}: {message}")
				}
			}
			Self::UnexpectedModule { locale, path } => {
				if let ::std::option::Option::Some(locale) = locale {
					write!(formatter, "{locale}: ")?;
				}

				write!(formatter, "unexpected module: {path}")
			}
			Self::DuplicateModule { locale, path } => {
				if let ::std::option::Option::Some(locale) = locale {
					write!(formatter, "{locale}: ")?;
				}

				write!(formatter, "duplicate module: {path}")
			}
			Self::MissingModules { locale, paths } => {
				if let ::std::option::Option::Some(locale) = locale {
					write!(formatter, "{locale}: ")?;
				}

				write!(formatter, "missing modules: {}", paths.join(", "))
			}
			Self::MissingLocales(locales) => write!(formatter, "missing languages: {locales:?}"),
			Self::LocaleMismatch => formatter.write_str("ready modules use different languages"),
			Self::Manifest(error) => ::std::fmt::Display::fmt(error, formatter),
		}
	}
}

impl ::std::error::Error for LoadError {
	fn source(&self) -> ::std::option::Option<&(dyn ::std::error::Error + 'static)> {
		match self {
			Self::Manifest(error) => ::std::option::Option::Some(error),
			_ => ::std::option::Option::None,
		}
	}
}

impl ::std::convert::From<ManifestError> for LoadError {
	fn from(error: ManifestError) -> Self {
		Self::Manifest(error)
	}
}
