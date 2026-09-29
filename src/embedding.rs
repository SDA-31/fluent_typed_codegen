//! Build a deferred macro recipe; includes expand only at an explicit callsite.
use crate::{discovery::CatalogSources, tree::Node};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeSet;

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

	fn recipe(&self, node: &Node) -> TokenStream {
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
		quote!((#source_language, #default_language, #directory, &[#(#entries,)*]))
	}

	/// One implementation dispatcher for the types declared in this Rust module.
	pub fn namespace(&self, node: &Node) -> TokenStream {
		let root = node.path.is_empty();
		let scopes: Vec<_> = root
			.then_some(node)
			.into_iter()
			.chain(node.children.values())
			.collect();
		let names: Vec<_> = scopes
			.iter()
			.map(|scope| format_ident!("{}", scope.ty))
			.collect();
		let recipes = scopes.iter().map(|scope| self.recipe(scope));
		let choices = scopes
			.iter()
			.map(|scope| scope.ty.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		let diagnostic = format!(
			"Use a generated catalog name in this namespace: {choices}. Imported type names and renamed type paths are not supported as embedded selectors."
		);
		let manifest = root.then(|| {
			let recipe = self.recipe(node);
			let mut known = BTreeSet::new();
			collect_type_names(node, &mut known);
			let known = known.into_iter().map(|name| format_ident!("{name}"));

			quote! {
				__fluent_codegen::__define_embed_manifest!(#recipe; [#(#known),*]; $);
			}
		});

		quote! {
			__fluent_codegen::__define_embed_scopes!(#(#names => #recipes,)*; #diagnostic; $);
			#manifest
		}
	}
}

fn collect_type_names<'a>(node: &'a Node, names: &mut BTreeSet<&'a str>) {
	names.insert(&node.ty);

	for child in node.children.values() {
		collect_type_names(child, names);
	}
}
