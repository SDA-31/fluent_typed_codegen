use super::Fixture;
use crate::{Extension, Scope, generate_with};
use quote::quote;
use std::{cell::RefCell, fs};
use syn::{Attribute, Item, ItemUse, parse_quote};

#[derive(Default)]
struct SyntaxExtension {
	scopes: RefCell<Vec<Scope>>,
}

impl Extension for SyntaxExtension {
	fn filename(&self) -> &str {
		"syntax_adapter.rs"
	}

	fn root_imports(&self) -> Vec<ItemUse> {
		vec![parse_quote!(
			use ::std::marker::PhantomData as AdapterMarker;
		)]
	}

	fn scope_imports(&self) -> Vec<ItemUse> {
		vec![parse_quote!(
			use super::AdapterMarker;
		)]
	}

	fn type_attributes(&self) -> Vec<Attribute> {
		vec![parse_quote!(#[allow(dead_code)])]
	}

	fn root_items(&self, scopes: &[Scope]) -> Vec<Item> {
		self.scopes.replace(scopes.to_vec());

		vec![parse_quote! {
			pub fn adapter_marker() -> AdapterMarker<()> {
				AdapterMarker
			}
		}]
	}
}

#[test]
fn extension_receives_typed_paths_and_raw_accessors_in_publication_order() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let extension = SyntaxExtension::default();
	let output = fixture.0.join("target/quoted \"output\" path");

	for language in ["de", "fr", "pt-BR"] {
		for module in ["type/match", "type/nested/my-hud"] {
			fixture.write(
				&format!("data/strings/languages/{language}/{module}.ftl"),
				"title = Example\n",
			);
		}
	}

	generate_with(&fixture.0, &output, &settings, &extension).unwrap();
	let scopes = extension.scopes.borrow();
	let expected: Vec<syn::Path> = vec![
		parse_quote!(Translations),
		parse_quote!(Type),
		parse_quote!(r#type::Nested),
		parse_quote!(r#type::nested::MyHud),
		parse_quote!(r#type::Match),
		parse_quote!(Ui),
		parse_quote!(ui::Main),
	];
	assert_eq!(
		scopes
			.iter()
			.map(|scope| scope.type_path.clone())
			.collect::<Vec<_>>(),
		expected
	);
	let accessors: Vec<syn::Ident> = vec![
		parse_quote!(r#type),
		parse_quote!(nested),
		parse_quote!(my_hud),
	];
	assert_eq!(scopes[3].accessors, accessors);
	assert!(scopes[0].accessors.is_empty());

	let source = fs::read_to_string(output.join(extension.filename())).unwrap();
	let file = syn::parse_file(&source).unwrap();
	let expected_import: Item = parse_quote!(
		use ::std::marker::PhantomData as AdapterMarker;
	);
	assert_eq!(file.items[0], expected_import);
	assert!(
		file.items.iter().any(
			|item| matches!(item, Item::Fn(function) if function.sig.ident == "adapter_marker")
		)
	);
	assert_scope_annotations(&file.items, false);
	assert!(
		source.lines().count() > 100,
		"generated file must remain readable"
	);
}

fn assert_scope_annotations(items: &[Item], nested: bool) {
	if nested {
		let import: Item = parse_quote!(
			use super::AdapterMarker;
		);
		assert!(items.contains(&import));
	}

	for item in items {
		match item {
			Item::Struct(ty) => {
				let attribute: Attribute = parse_quote!(#[allow(dead_code)]);
				assert!(ty.attrs.contains(&attribute));
			}
			Item::Mod(module)
				if matches!(
					module.ident.to_string().as_str(),
					"r#type" | "nested" | "ui"
				) =>
			{
				let (_, items) = module.content.as_ref().unwrap();
				assert_scope_annotations(items, true);
			}
			_ => {}
		}
	}
}

#[test]
fn invalid_verbatim_extension_syntax_is_reported_before_writing_its_entrypoint() {
	struct InvalidSyntax;

	impl Extension for InvalidSyntax {
		fn filename(&self) -> &str {
			"invalid_adapter.rs"
		}

		fn root_items(&self, _: &[Scope]) -> Vec<Item> {
			vec![Item::Verbatim(quote!(pub fn broken(value: ) {}))]
		}
	}

	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");
	let error = generate_with(&fixture.0, &output, &settings, &InvalidSyntax).unwrap_err();

	assert!(
		error.contains("generated Rust `invalid_adapter.rs`"),
		"{error}"
	);
	assert!(!output.join("invalid_adapter.rs").exists());
}

#[test]
fn declaration_hooks_preserve_complete_types_without_changing_plain_output() {
	struct Wrapped;

	impl Extension for Wrapped {
		fn filename(&self) -> &str {
			"wrapped.rs"
		}

		fn type_attributes(&self) -> Vec<Attribute> {
			vec![parse_quote!(#[allow(dead_code)])]
		}

		fn type_declaration(&self, declaration: syn::ItemStruct) -> Item {
			let cloned: Attribute = parse_quote!(#[derive(Clone)]);
			let allowed: Attribute = parse_quote!(#[allow(dead_code)]);
			assert!(declaration.attrs.contains(&cloned));
			assert!(declaration.attrs.contains(&allowed));
			assert!(
				declaration
					.attrs
					.iter()
					.any(|attr| attr.path().is_ident("doc"))
			);

			parse_quote!(runtime::declare! { #declaration })
		}

		fn root_items(&self, _: &[Scope]) -> Vec<Item> {
			Vec::new()
		}
	}

	fn wrapped_types(items: &[Item], names: &mut Vec<String>) {
		for item in items {
			match item {
				Item::Macro(item) if item.mac.path == parse_quote!(runtime::declare) => {
					let declaration: syn::ItemStruct =
						syn::parse2(item.mac.tokens.clone()).unwrap();
					names.push(declaration.ident.to_string());
				}
				Item::Mod(module) => {
					if let Some((_, children)) = &module.content {
						wrapped_types(children, names);
					}
				}
				_ => {}
			}
		}
	}

	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/wrapped");
	generate_with(&fixture.0, &output, &settings, &Wrapped).unwrap();
	let wrapped = syn::parse_file(&fs::read_to_string(output.join("wrapped.rs")).unwrap()).unwrap();
	let mut names = Vec::new();
	wrapped_types(&wrapped.items, &mut names);
	assert_eq!(names, ["Translations", "Ui", "Main"]);
	let plain = fs::read_to_string(output.join("translations.rs")).unwrap();
	assert!(!plain.contains("runtime::declare"));
	assert!(plain.contains("pub struct Translations"));
}
