//! Compile discovered asset addresses and catalog settings into Rust constants.
use crate::{Settings, discovery::CatalogSources};
use proc_macro2::TokenStream;
use quote::quote;
use std::path::Path;

pub(super) fn render(
	settings: &Settings,
	sources: &CatalogSources,
	config_path: &Path,
) -> Result<TokenStream, String> {
	let asset_root = settings
		.asset_root
		.to_str()
		.ok_or("asset-root must be UTF-8")?;
	let catalog = settings.catalog.to_str().ok_or("catalog must be UTF-8")?;
	let default_language = &sources.configuration.default_language;
	let source_language = &sources.configuration.source_language;
	let languages_directory = sources
		.configuration
		.languages_directory
		.to_str()
		.ok_or("translations-directory must be UTF-8")?;
	let configuration = config_path.to_str().ok_or("catalog path must be UTF-8")?;
	let mut modules = Vec::new();

	for module in &sources.modules {
		let language = &module.language;
		let path = &module.path;
		modules.push(quote!((#language, #path)));
	}

	Ok(quote! {
		/// Build-time asset root relative to the consuming Cargo package.
		/// Runtime storage may use a different root or a virtual asset source.
		pub const ASSET_ROOT: &str = #asset_root;
		/// Definition asset path relative to ASSET_ROOT.
		pub const CATALOG_ASSET_PATH: &str = #catalog;
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
