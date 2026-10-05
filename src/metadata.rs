//! Compile filesystem input locations and catalog settings into Rust constants.
use crate::{BuildError, Settings, discovery::CatalogSources};
use proc_macro2::TokenStream;
use quote::quote;
use std::path::Path;

pub(super) fn render(
	settings: &Settings,
	sources: &CatalogSources,
	config_path: &Path,
) -> Result<TokenStream, BuildError> {
	let catalog = BuildError::utf8(&settings.catalog)?;
	let default_language = &sources.configuration.default_language;
	let source_language = &sources.configuration.source_language;
	let languages_directory = BuildError::utf8(&sources.configuration.languages_directory)?;
	let configuration = BuildError::utf8(config_path)?;
	let mut modules = Vec::new();

	for module in &sources.modules {
		let language = &module.language;
		let path = &module.path;
		modules.push(quote!((#language, #path)));
	}

	Ok(quote! {
		/// Build-time TOML path relative to the consuming Cargo package.
		/// Runtime filesystem locations and engine asset addresses are application-owned.
		pub const CATALOG_PATH: &str = #catalog;
		/// Configured startup language, independent of the typed API's source language.
		pub const DEFAULT_LANGUAGE: &str = #default_language;
		/// Language whose messages and type annotations define the generated API.
		pub const SOURCE_LANGUAGE: &str = #source_language;
		/// Resolved translations-directory relative to the definition's parent; defaults to `.`.
		pub const LANGUAGES_DIRECTORY: &str = #languages_directory;
		/// Unmodified definition text embedded at build time.
		pub const CATALOG_CONFIG: &str = include_str!(#configuration);
		/// Known modules as (locale code, logical path below that locale), without FTL data.
		/// Module paths are logical addresses, independent of the runtime storage format.
		pub const MODULES: &[(&str, &str)] = &[#(#modules,)*];
	})
}
