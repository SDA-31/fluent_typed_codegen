//! One locale selector for every generated module; no fixed language allowlist.
use crate::{discovery::CatalogSources, tree::names};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;

pub(super) fn render(sources: &CatalogSources) -> Result<TokenStream, String> {
	let mut variants = BTreeMap::new();

	for module in &sources.modules {
		let (_, variant) = names(&module.language)?;

		if let Some(previous) = variants.insert(variant.clone(), module.language.clone())
			&& previous != module.language
		{
			return Err(format!(
				"locale names `{previous}` and `{}` collide as Rust variant `{variant}`",
				module.language
			));
		}
	}

	let (_, default) = names(&sources.configuration.source_language)?;
	let identifiers: Vec<_> = variants
		.keys()
		.map(|variant| format_ident!("{variant}"))
		.collect();
	let languages: Vec<_> = variants.values().collect();
	let declarations =
		variants
			.keys()
			.zip(&identifiers)
			.zip(&languages)
			.map(|((variant, ident), language)| {
				let description = format!("`{language}`.");
				let default_attribute = (variant == &default).then(|| quote!(#[default]));

				quote! {
					#[doc = #description]
					#default_attribute
					#ident,
				}
			});

	Ok(quote! {
		/// All discovered languages. Default identifies the source language, not the startup policy.
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
		pub enum Locale {
			#(#declarations)*
		}

		impl ::std::convert::AsRef<str> for Locale {
			fn as_ref(&self) -> &str {
				match self {
					#(Self::#identifiers => #languages,)*
				}
			}
		}

		impl ::std::fmt::Display for Locale {
			fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
				f.write_str(self.as_ref())
			}
		}

		impl ::std::str::FromStr for Locale {
			type Err = ::std::string::String;

			fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
				match value {
					#(#languages => ::std::result::Result::Ok(Self::#identifiers),)*
					_ => ::std::result::Result::Err(format!("unknown compiled language: {value}")),
				}
			}
		}

		impl Locale {
			/// Iterate over compiled languages.
			pub fn iter() -> ::std::slice::Iter<'static, Self> {
				[#(Self::#identifiers,)*].iter()
			}

			/// Load the full tree of embedded catalogs for this language.
			pub fn load(self) -> Translations {
				Translations::embedded(self)
			}
		}
	})
}
