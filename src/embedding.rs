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

	fn declaration_recipes(&self, node: &Node, namespace: &TokenStream) -> TokenStream {
		let ty = format_ident!("{}", node.ty);
		let recipe = self.recipe(node);
		let child_namespace = if node.path.is_empty() {
			namespace.clone()
		} else {
			let name = format_ident!("{}", node.name);
			quote!(#namespace #name ::)
		};
		let children = node
			.children
			.values()
			.map(|child| self.declaration_recipes(child, &child_namespace));

		quote! {
			(#namespace #ty) => #recipe,
			#(#children)*
		}
	}

	/// One deferred manifest declaration macro for the complete translation tree.
	pub fn manifest(&self, root: &Node) -> TokenStream {
		let recipe = self.recipe(root);
		let selectors = selector_items(root);
		let declarations = self.declaration_recipes(root, &TokenStream::new());

		quote! {
			__fluent_codegen::__define_embed_manifest!(
				#recipe; { #selectors }; { #declarations }; $
			);
		}
	}
}

/// A scope-only namespace local to each declaration, independent of application imports.
fn selector_items(node: &Node) -> TokenStream {
	let ty = format_ident!("{}", node.ty);
	let description = if node.path.is_empty() {
		"Select all translation modules in all discovered languages.".into()
	} else if node.leaf.is_some() {
		format!("Select `{}.ftl` in all discovered languages.", node.path)
	} else {
		format!(
			"Select every module below `{}/` in all discovered languages.",
			node.path
		)
	};
	let children = node.children.values().map(selector_items);
	let nested = if node.path.is_empty() {
		quote!(#(#children)*)
	} else if node.leaf.is_none() {
		let name = format_ident!("{}", node.name);
		quote!(pub mod #name { #(#children)* })
	} else {
		TokenStream::new()
	};

	quote! {
		#[doc = #description]
		pub struct #ty;
		#nested
	}
}
