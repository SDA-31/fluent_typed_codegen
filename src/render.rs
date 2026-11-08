//! Emit independent catalog/group wrappers around upstream typed message APIs.
use crate::{
	BuildError, Extension, Scope, build_io, discovery::CatalogSources, locale,
	message_types::MessageTypes, schema, tree::Node,
};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use syn::{Ident, Path, parse_quote};

pub(super) fn render(
	root: &Node,
	sources: &CatalogSources,
	message_types: &MessageTypes,
	extension: Option<&dyn Extension>,
) -> Result<TokenStream, BuildError> {
	let imports = extension
		.map(|extension| extension.root_imports())
		.unwrap_or_default();
	let locale = locale::render(sources)?;
	let embedding = crate::embedding::Recipes::new(sources)?.manifest(root);
	let mut schemas = BTreeMap::new();

	for module in sources
		.modules
		.iter()
		.filter(|module| module.language == sources.configuration.source_language)
	{
		let source = build_io::read_to_string(&module.absolute)?;
		schemas.insert(
			module.path.trim_end_matches(".ftl").to_owned(),
			schema::schema(&source).map_err(|source| BuildError::Schema {
				path: module.absolute.clone(),
				locale: module.language.clone(),
				source,
			})?,
		);
	}

	let tree = render_node(root, true, message_types, &schemas, extension);
	let errors = syn::parse_file(include_str!("../templates/error.rs")).map_err(|source| {
		BuildError::Syntax {
			file: "templates/error.rs".into(),
			source,
		}
	})?;
	let catalog = syn::parse_file(include_str!("../templates/catalog.rs")).map_err(|source| {
		BuildError::Syntax {
			file: "templates/catalog.rs".into(),
			source,
		}
	})?;
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
			CATALOG_PATH, DEFAULT_LANGUAGE, SOURCE_LANGUAGE,
			LANGUAGES_DIRECTORY, CATALOG_CONFIG, MODULES,
		};

		mod __validation {
			include!("validation.rs");
		}

		/// Shared source contract and I/O error types, independent of this generated schema.
		#[doc(inline)]
		#[allow(unused_imports)]
		pub use __fluent_codegen::{LocalizationManifest, ManifestError};

		#locale
		#errors
		#embedding
		#tree
		#catalog
		#(#items)*
	})
}

fn render_node(
	node: &Node,
	root: bool,
	message_types: &MessageTypes,
	schemas: &BTreeMap<String, schema::Schema>,
	extension: Option<&dyn Extension>,
) -> TokenStream {
	let ty = identifier(&node.ty);
	let name = (!root).then(|| identifier(&node.name));
	let implementation = identifier(&format!("__leaf_{}", node.name.trim_start_matches("r#")));
	let visibility = if node.path.contains('/') {
		quote!(pub(super))
	} else {
		quote!()
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
	let (definition, constructors, external, validation) = if node.leaf.is_some() {
		let module = format!("{}.ftl", node.path);
		let entries = schemas[&node.path].iter().map(|(key, references)| {
			let references: Vec<_> = references.iter().collect();
			quote!((#key, &[#(#references,)*]))
		});
		(
			quote! {
				pub struct #ty(::std::sync::Arc<#implementation::L10nLanguage>, Locale);
			},
			crate::leaf_constructors::render(&implementation, &module, entries.collect()),
			quote! {
				let source = sources.get(#module)
					.ok_or_else(|| LoadError::MissingModules {
						locale: ::std::option::Option::Some(locale), paths: ::std::vec![#module.into()],
					})?;

				if checked {
					Self::new(locale, source.as_bytes())
				} else {
					Self::new_unchecked(locale, source.as_bytes())
				}
			},
			quote! {
				let source = sources.get(#module)
					.ok_or_else(|| LoadError::MissingModules {
						locale: ::std::option::Option::None, paths: ::std::vec![#module.into()],
					})?;
				Self::validate(source.as_bytes())
			},
		)
	} else {
		(
			quote! {
				pub struct #ty {
					locale: Locale,
					#(#fields: #types,)*
				}
			},
			quote! {
				/// Language represented by this complete, immutable scope.
				pub fn locale(&self) -> Locale {
					self.locale
				}

				/// Assemble a complete scope from already loaded immediate children.
				/// This integration hook shares their resources without rereading or parsing.
				#[doc(hidden)]
				pub fn __from_parts(locale: Locale, #(#fields: #types,)*) -> ::std::result::Result<Self, LoadError> {
					if false #(|| #fields.locale() != locale)* {
						return ::std::result::Result::Err(LoadError::LocaleMismatch);
					}

					::std::result::Result::Ok(Self { locale, #(#fields,)* })
				}
			},
			quote! {
				::std::result::Result::Ok(Self {
					locale,
					#(#fields: #types::__external(locale, sources, checked)?,)*
				})
			},
			quote! {
				#(#types::__validate(sources)?;)*
				::std::result::Result::Ok(())
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
		#[derive(Debug)]
		#(#attributes)*
		#definition
	};
	let declaration = match extension {
		Some(extension) => extension.type_declaration(declaration),
		None => syn::Item::Struct(declaration),
	};

	let owned = if node.leaf.is_some() {
		let module = format!("{}.ftl", node.path);

		quote! {
			let bytes = sources.remove(#module).ok_or_else(|| LoadError::MissingModules {
				locale: ::std::option::Option::Some(locale), paths: ::std::vec![#module.into()],
			})?;

			if checked {
				Self::new(locale, bytes.into_owned())
			} else {
				Self::new_unchecked(locale, bytes.into_owned())
			}
		}
	} else {
		quote! {
			::std::result::Result::Ok(Self {
				locale,
				#(#fields: #types::__from_buffers(locale, sources, checked)?,)*
			})
		}
	};

	let declarations = quote! {
		#declaration

		impl #ty {
			#(#accessors)*

			#constructors

			#visibility fn __external(
				locale: Locale,
				sources: &::std::collections::BTreeMap<&str, &str>,
				checked: bool,
			) -> ::std::result::Result<Self, LoadError> {
				#external
			}

			#visibility fn __from_buffers(
				locale: Locale,
				sources: &mut ::std::collections::BTreeMap<::std::string::String, ::std::borrow::Cow<'_, [u8]>>,
				checked: bool,
			) -> ::std::result::Result<Self, LoadError> {
				#owned
			}

			#visibility fn __validate(sources: &::std::collections::BTreeMap<&str, &str>) -> ::std::result::Result<(), LoadError> {
				#validation
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

	let children = node
		.children
		.values()
		.map(|child| render_node(child, false, message_types, schemas, extension));

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
			use super::{fluent_typed, Locale, LoadError, LocalizationManifest, __validation};
			#(#imports)*
			#(#children)*
		}
	}
}

pub(super) fn validate_extension(
	node: &Node,
	extension: Option<&dyn Extension>,
) -> Result<(), BuildError> {
	let Some(extension) = extension else {
		return Ok(());
	};

	let name = node.name.strip_prefix("r#").unwrap_or(&node.name);

	if extension.reserved_names().contains(&name) {
		return Err(BuildError::ReservedName {
			path: node.path.clone(),
			name: node.name.clone(),
			ty: node.ty.clone(),
			extension: true,
		});
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
		logical_path: node.path.clone(),
		module_path: node.leaf.map(|_| format!("{}.ftl", node.path)),
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
