use super::Fixture;
use crate::generate;
use std::fs;
use syn::{Attribute, ImplItem, Item, Meta, Visibility};

#[test]
fn generated_facade_and_metadata_keep_public_rustdoc() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");

	for language in ["de", "fr", "pt-BR"] {
		fixture.write(
			&format!("data/strings/languages/{language}/ui/hud.ftl"),
			"# $icon (Element) - Inline icon.\nprompt = Press { $icon }\n",
		);
	}

	generate(&fixture.0, &output, &settings).unwrap();

	for file in ["translations.rs", "locale_modules.rs"] {
		let source = fs::read_to_string(output.join(file)).unwrap();
		let syntax = syn::parse_file(&source).unwrap();
		assert_documented(&syntax.items);
	}
}

fn assert_documented(items: &[Item]) {
	for item in items {
		let (visibility, attributes) = match item {
			Item::Const(item) => (&item.vis, &item.attrs),
			Item::Struct(item) => (&item.vis, &item.attrs),
			Item::Enum(item) => {
				for variant in &item.variants {
					assert!(
						has_prose(&variant.attrs),
						"undocumented variant: {variant:?}"
					);
				}

				(&item.vis, &item.attrs)
			}
			Item::Mod(item) => {
				if matches!(item.vis, Visibility::Public(_))
					&& let Some((_, contents)) = &item.content
				{
					assert_documented(contents);
				}

				(&item.vis, &item.attrs)
			}
			Item::Use(item)
				if item
					.attrs
					.iter()
					.any(|attr| attr.meta == syn::parse_quote!(doc(inline))) =>
			{
				(&item.vis, &item.attrs)
			}
			Item::Impl(item) if item.trait_.is_none() => {
				for method in &item.items {
					let ImplItem::Fn(method) = method else {
						continue;
					};

					if matches!(method.vis, Visibility::Public(_)) {
						assert!(has_prose(&method.attrs), "undocumented method: {method:?}");
					}
				}

				continue;
			}
			_ => continue,
		};

		if matches!(visibility, Visibility::Public(_)) {
			assert!(
				has_prose(attributes),
				"undocumented generated item: {item:?}"
			);
		}
	}
}

fn has_prose(attributes: &[Attribute]) -> bool {
	attributes.iter().any(|attribute| {
		let Meta::NameValue(meta) = &attribute.meta else {
			return false;
		};

		meta.path.is_ident("doc")
			&& matches!(&meta.value, syn::Expr::Lit(value) if matches!(&value.lit, syn::Lit::Str(text) if !text.value().trim().is_empty()))
	})
}
