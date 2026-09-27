//! Build a deferred macro recipe; includes expand only at an explicit callsite.
use crate::discovery::CatalogSources;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;

pub(super) fn render(sources: &CatalogSources) -> Result<TokenStream, String> {
	let config = &sources.configuration;
	let source_language = &config.source_language;
	let default_language = &config.default_language;
	let directory = config
		.languages_directory
		.to_str()
		.ok_or("translations-directory must be UTF-8")?;
	let mut entries = Vec::new();
	let mut modules: BTreeMap<&str, Vec<TokenStream>> = BTreeMap::new();

	for module in &sources.modules {
		let language = &module.language;
		let path = &module.path;
		let absolute = module
			.absolute
			.to_str()
			.ok_or("module path must be UTF-8")?;
		let entry = quote!((#language, #path, ::std::include_bytes!(#absolute) as &'static [u8]));
		entries.push(entry.clone());
		modules.entry(path).or_default().push(entry);
	}

	let module_recipes = modules.iter().map(|(path, entries)| {
		quote! {
			#path => (#source_language, #default_language, #directory, &[#(#entries,)*])
		}
	});

	Ok(quote! {
		__fluent_codegen::__define_embed_manifest!(
			(#source_language, #default_language, #directory, &[#(#entries,)*]);
			#(#module_recipes,)*
		);
	})
}
