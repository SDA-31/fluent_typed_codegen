//! Emit named catalog/group wrappers around unmodified upstream typed APIs.
use crate::{
	Extension, Scope, discovery::CatalogSources, locale, message_types::MessageTypes, tree::Node,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path, Visibility, parse_quote};

pub(super) fn render(
	root: &Node,
	sources: &CatalogSources,
	message_types: &MessageTypes,
	extension: Option<&dyn Extension>,
) -> Result<TokenStream, String> {
	let imports = extension
		.map(|extension| extension.root_imports())
		.unwrap_or_default();
	let locale = locale::render(sources)?;
	let tree = render_node(root, true, 0, message_types, extension);
	let catalog = syn::parse_file(include_str!("../templates/catalog.rs"))
		.map_err(|error| format!("catalog template: {error}"))?;
	let mut scopes = Vec::new();
	collect_scopes(root, &[], &[], &mut scopes);
	let items = extension
		.map(|extension| extension.root_items(&scopes))
		.unwrap_or_default();

	Ok(quote! {
		#(#imports)*

		mod __catalog_metadata {
			include!("locale_modules.rs");
		}

		#[allow(unused_imports)]
		pub use __catalog_metadata::{
			ASSET_ROOT, CATALOG_ASSET_PATH, DEFAULT_LANGUAGE, SOURCE_LANGUAGE,
			LANGUAGES_DIRECTORY, CATALOG_CONFIG, MODULES,
		};

		mod __validation {
			include!("validation.rs");
		}

		#locale
		#tree
		#catalog
		#(#items)*
	})
}

fn render_node(
	node: &Node,
	root: bool,
	depth: usize,
	message_types: &MessageTypes,
	extension: Option<&dyn Extension>,
) -> TokenStream {
	let ty = identifier(&node.ty);
	let name = (!root).then(|| identifier(&node.name));
	let implementation = identifier(&format!("__leaf_{}", node.name.trim_start_matches("r#")));
	let visibility: Visibility = if depth == 0 {
		Visibility::Inherited
	} else {
		parse_quote!(pub(super))
	};

	let kind = if node.leaf.is_some() {
		"catalog"
	} else {
		"group"
	};
	let path = if root { "root" } else { &node.path };
	let description = format!(
		"Typed localization {kind} `{path}`.\n\nClones share immutable parsed leaf catalogs through Arc; cloning does not parse or reload sources."
	);
	let attributes = extension
		.map(|extension| extension.type_attributes())
		.unwrap_or_default();
	let fields: Vec<_> = node
		.children
		.values()
		.map(|child| identifier(&child.name))
		.collect();
	let types: Vec<Path> = node
		.children
		.values()
		.map(|child| {
			let child_type = identifier(&child.ty);

			if root {
				parse_quote!(#child_type)
			} else {
				parse_quote!(#name::#child_type)
			}
		})
		.collect();
	let locale_field = root.then(|| quote!(locale: Locale,));
	let locale_value = root.then(|| quote!(locale,));

	let (definition, embedded, external) = if node.leaf.is_some() {
		let module = format!("{}.ftl", node.path);
		(
			quote! {
				pub struct #ty(::std::sync::Arc<#implementation::L10nLanguage>);
			},
			quote! {
				Self(::std::sync::Arc::new(
					locale.as_ref().parse::<#implementation::L10n>()
						.expect("build-validated module locale").load()
				))
			},
			quote! {
				let source = sources.get(#module)
					.ok_or_else(|| format!("missing module: {}", #module))?;

				#implementation::L10nLanguage::new_external(locale, source.as_bytes())
					.map(|catalog| Self(::std::sync::Arc::new(catalog)))
					.map_err(|error| format!("{}: {error}", #module))
			},
		)
	} else {
		(
			quote! {
				pub struct #ty {
					#locale_field
					#(#fields: #types,)*
				}
			},
			quote! {
				Self {
					#locale_value
					#(#fields: #types::__embedded(locale),)*
				}
			},
			quote! {
				::std::result::Result::Ok(Self {
					#locale_value
					#(#fields: #types::__external(locale, sources)?,)*
				})
			},
		)
	};

	let accessors = node
		.children
		.values()
		.zip(&fields)
		.zip(&types)
		.map(|((child, field), ty)| {
			let description = format!("Borrow the `{}` localization scope.", child.path);

			quote! {
				#[doc = #description]
				// Accessor names mirror FTL paths, including as-ref.ftl.
				#[allow(clippy::should_implement_trait)]
				pub fn #field(&self) -> &#ty {
					&self.#field
				}
			}
		});

	let declaration = parse_quote! {
		#[doc = #description]
		#[derive(Clone)]
		#(#attributes)*
		#definition
	};
	let declaration = match extension {
		Some(extension) => extension.type_declaration(declaration),
		None => syn::Item::Struct(declaration),
	};

	let declarations = quote! {
		#declaration

		impl #ty {
			#(#accessors)*

			#visibility fn __embedded(locale: Locale) -> Self {
				#embedded
			}

			#visibility fn __external(
				locale: Locale,
				sources: &::std::collections::BTreeMap<&str, &str>,
			) -> ::std::result::Result<Self, ::std::string::String> {
				#external
			}
		}
	};

	if node.leaf.is_some() {
		let source = format!("modules/{}/translations.rs", node.path);
		let exports = message_types[&node.path].iter().map(|export| {
			let original = &export.original;
			let alias = &export.alias;
			let description = format!(
				"Upstream message type `{original}` from `{}.ftl`, named with its leaf prefix.",
				node.path
			);

			quote! {
				#[doc = #description]
				#[doc(inline)]
				#[allow(unused_imports)]
				pub use #implementation::#original as #alias;
			}
		});

		return quote! {
			#declarations

			impl ::std::ops::Deref for #ty {
				type Target = #implementation::L10nLanguage;

				fn deref(&self) -> &Self::Target {
					&self.0
				}
			}

			mod #implementation {
				use super::fluent_typed;
				include!(#source);
			}

			#(#exports)*
		};
	}

	let children = node.children.values().map(|child| {
		render_node(
			child,
			false,
			if root { depth } else { depth + 1 },
			message_types,
			extension,
		)
	});

	if root {
		return quote! {
			#declarations
			#(#children)*
		};
	}

	let imports = extension
		.map(|extension| extension.scope_imports())
		.unwrap_or_default();
	let description = format!("Nested catalogs below `{}`.", node.path);

	quote! {
		#declarations

		#[doc = #description]
		pub mod #name {
			use super::{fluent_typed, Locale};
			#(#imports)*
			#(#children)*
		}
	}
}

pub(super) fn validate_extension(
	node: &Node,
	extension: Option<&dyn Extension>,
) -> Result<(), String> {
	let Some(extension) = extension else {
		return Ok(());
	};

	let name = node.name.strip_prefix("r#").unwrap_or(&node.name);

	if extension.reserved_names().contains(&name) {
		return Err(format!(
			"{}: name `{}` is reserved by the extension",
			node.path, node.name
		));
	}

	for child in node.children.values() {
		validate_extension(child, Some(extension))?;
	}

	Ok(())
}

fn collect_scopes(node: &Node, namespace: &[Ident], accessors: &[Ident], scopes: &mut Vec<Scope>) {
	let ty = identifier(&node.ty);
	scopes.push(Scope {
		type_path: parse_quote!(#(#namespace::)* #ty),
		accessors: accessors.to_vec(),
	});
	let mut child_namespace = namespace.to_vec();

	if !node.name.is_empty() {
		child_namespace.push(identifier(&node.name));
	}

	for child in node.children.values() {
		let mut child_accessors = accessors.to_vec();
		child_accessors.push(identifier(&child.name));
		collect_scopes(child, &child_namespace, &child_accessors, scopes);
	}
}

fn identifier(name: &str) -> Ident {
	syn::parse_str(name).expect("tree::names validated the generated Rust identifier")
}
