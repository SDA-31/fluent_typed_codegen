//! Build a deferred macro recipe; includes expand only at an explicit callsite.
use crate::{discovery::CatalogSources, tree::Node};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) struct Recipes<'a> {
	sources: &'a CatalogSources,
	directory: &'a str,
	entries: Vec<(&'a str, TokenStream)>,
}

impl<'a> Recipes<'a> {
	pub fn new(sources: &'a CatalogSources) -> Result<Self, String> {
		let directory = sources
			.configuration
			.languages_directory
			.to_str()
			.ok_or("translations-directory must be UTF-8")?;
		let mut entries = Vec::new();

		for module in &sources.modules {
			let language = &module.language;
			let path = &module.path;
			let absolute = module
				.absolute
				.to_str()
				.ok_or("module path must be UTF-8")?;
			let entry =
				quote!((#language, #path, ::std::include_bytes!(#absolute) as &'static [u8]));
			entries.push((path.as_str(), entry));
		}

		Ok(Self {
			sources,
			directory,
			entries,
		})
	}

	pub fn scope(&self, node: &Node) -> TokenStream {
		let source_language = &self.sources.configuration.source_language;
		let default_language = &self.sources.configuration.default_language;
		let directory = self.directory;
		let leaf = format!("{}.ftl", node.path);
		let prefix = format!("{}/", node.path);
		let entries = self.entries.iter().filter_map(|(path, entry)| {
			(node.path.is_empty()
				|| (node.leaf.is_some() && *path == leaf)
				|| (node.leaf.is_none() && path.starts_with(&prefix)))
			.then_some(entry)
		});
		let recipe = quote!((#source_language, #default_language, #directory, &[#(#entries,)*]));
		let ty = format_ident!("{}", node.ty);
		let internal = format_ident!("__fluent_embed_{}", node.ty);
		let root = node.path.is_empty().then(|| {
			quote! {
				__fluent_codegen::__define_embed_manifest!(#recipe; $);
			}
		});

		quote! {
			__fluent_codegen::__define_embed_scope!(#ty, #internal, #recipe);
			#root
		}
	}
}
