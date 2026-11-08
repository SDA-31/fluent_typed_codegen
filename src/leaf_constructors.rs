//! Constructors preserve source ownership until the Fluent resource takes its buffer.
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub(super) fn render(
	implementation: &Ident,
	module: &str,
	entries: Vec<TokenStream>,
) -> TokenStream {
	quote! {
		/// Logical FTL path below a locale, independent of runtime storage.
		pub const PATH: &'static str = #module;

		/// Language represented by this immutable module.
		pub fn locale(&self) -> Locale {
			self.1
		}

		/// Validate and parse FTL, taking ownership of its source buffer.
		///
		/// `Vec<u8>` and `String` transfer their allocation; slices are copied once.
		/// Validation and parsing still allocate their own structures.
		///
		/// # Errors
		/// Rejects invalid or incompatible FTL. No input borrow is retained.
		pub fn new(locale: Locale, bytes: impl ::std::convert::Into<::std::vec::Vec<u8>>) -> ::std::result::Result<Self, LoadError> {
			let bytes = bytes.into();
			Self::__validate_for_locale(locale, &bytes)?;
			Self::new_unchecked(locale, bytes)
		}

		/// Parse FTL without checking its generated message contract.
		///
		/// `Vec<u8>` and `String` transfer their allocation; slices are copied once.
		/// This is safe Rust, but incompatible messages may cause accessor panics
		/// or formatting failures. No input borrow is retained.
		///
		/// # Errors
		/// Rejects invalid UTF-8 or Fluent syntax.
		pub fn new_unchecked(locale: Locale, bytes: impl ::std::convert::Into<::std::vec::Vec<u8>>) -> ::std::result::Result<Self, LoadError> {
			#implementation::L10nLanguage::new(locale, bytes)
				.map(|catalog| Self(::std::sync::Arc::new(catalog), locale))
				.map_err(|error| LoadError::Module {
					locale: ::std::option::Option::Some(locale), path: Self::PATH.into(), message: error.to_string(),
				})
		}

		/// Check the module contract without retaining a parsed catalog.
		///
		/// Parsing uses temporary allocations; a later constructor parses again.
		///
		/// # Errors
		/// Rejects invalid UTF-8, Fluent syntax or changed keys/references/contracts.
		pub fn validate(bytes: &[u8]) -> ::std::result::Result<(), LoadError> {
			let check = || -> ::std::result::Result<(), ::std::string::String> {
				let source = ::std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
				__validation::validate_schema(source, &[#(#entries,)*]).map_err(|error| error.to_string())?;
				fluent_typed::validate_ftl(bytes, #implementation::MESSAGE_CONTRACTS)
					.map_err(|error| error.to_string())
			};
			check().map_err(|message| LoadError::Module {
				locale: ::std::option::Option::None, path: Self::PATH.into(), message,
			})
		}

		fn __validate_for_locale(locale: Locale, bytes: &[u8]) -> ::std::result::Result<(), LoadError> {
			Self::validate(bytes).map_err(|error| match error {
				LoadError::Module { path, message, .. } => LoadError::Module {
					locale: ::std::option::Option::Some(locale), path, message,
				},
				other => other,
			})
		}

		/// Read and validate this module from an explicit file or embedded manifest.
		/// File buffers are consumed; embedded bytes are copied into the parsed resource.
		///
		/// # Errors
		/// Returns source, UTF-8, syntax or message-contract diagnostics.
		pub fn from_manifest(locale: Locale, manifest: &LocalizationManifest) -> ::std::result::Result<Self, LoadError> {
			Self::new(locale, manifest.read(locale.as_ref(), Self::PATH)?.into_owned())
		}
	}
}
