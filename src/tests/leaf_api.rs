use super::Fixture;
use crate::generate;
use std::fs;
use syn::{Item, UseTree, Visibility};

#[test]
fn leaves_export_types_not_modules_and_helpers_cannot_shadow_catalogs() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");

	for language in ["de", "fr", "pt-BR"] {
		for leaf in [
			"world",
			"type",
			"catalog-metadata",
			"validation",
			"fluent-runtime",
			"fluent-bridge",
		] {
			fixture.write(
				&format!("data/strings/languages/{language}/{leaf}.ftl"),
				"title = Example\n",
			);
		}
	}

	generate(&fixture.0, &output, &settings).unwrap();
	let source = fs::read_to_string(output.join("translations.rs")).unwrap();
	let tree = syn::parse_file(&source).unwrap();
	let public_modules: Vec<_> = tree.items.iter().filter_map(public_module_name).collect();

	assert_eq!(public_modules, ["ui"]);
	assert!(
		tree.items
			.iter()
			.any(|item| matches!(item, Item::Struct(ty) if ty.ident == "World"))
	);
	assert!(source.contains("mod __leaf_catalog_metadata"));
	assert!(source.contains("mod __leaf_validation"));
	assert!(source.contains("mod __leaf_type"));
	assert!(source.contains("mod __leaf_fluent_runtime"));
	assert!(source.contains("mod __leaf_fluent_bridge"));
	assert!(!source.contains("pub mod main"));
	assert!(!source.contains("pub mod world"));
}

#[test]
fn structured_outputs_have_leaf_prefixed_public_type_names() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");

	for language in ["de", "fr", "pt-BR"] {
		for leaf in ["first", "second"] {
			fixture.write(
				&format!("data/strings/languages/{language}/ui/{leaf}.ftl"),
				"# $icon (Element) - Inline icon.\nprompt = Press { $icon }\n",
			);
		}
	}

	generate(&fixture.0, &output, &settings).unwrap();
	let source = fs::read_to_string(output.join("translations.rs")).unwrap();
	let tree = syn::parse_file(&source).unwrap();
	let ui = tree
		.items
		.iter()
		.find_map(|item| match item {
			Item::Mod(module) if module.ident == "ui" => module.content.as_ref(),
			_ => None,
		})
		.unwrap();
	let aliases: Vec<_> =
		ui.1.iter()
			.filter_map(|item| {
				let Item::Use(import) = item else {
					return None;
				};
				let UseTree::Path(path) = &import.tree else {
					return None;
				};
				let UseTree::Rename(rename) = path.tree.as_ref() else {
					return None;
				};

				matches!(import.vis, Visibility::Public(_)).then(|| rename.rename.to_string())
			})
			.collect();

	assert_eq!(aliases, ["FirstPrompt", "SecondPrompt"]);
	assert!(!ui.1.iter().any(|item| public_module_name(item).is_some()));
}

#[test]
fn payload_alias_collisions_report_both_source_paths() {
	for sibling in ["first-prompt-name", "first-prompt"] {
		let fixture = Fixture::new();
		let settings = fixture.catalogs();
		let output = fixture.0.join("target/generated");

		for language in ["de", "fr", "pt-BR"] {
			fixture.write(
				&format!("data/strings/languages/{language}/ui/first.ftl"),
				"# $icon (Element) - Inline icon.\nprompt-name = Press { $icon }\n",
			);
			fixture.write(
				&format!("data/strings/languages/{language}/ui/{sibling}.ftl"),
				"# $icon (Element) - Inline icon.\nname = Press { $icon }\n",
			);
		}

		let error = generate(&fixture.0, &output, &settings).unwrap_err();

		assert!(error.contains("FirstPromptName"), "{error}");
		assert!(error.contains("ui/first.ftl"), "{error}");
		assert!(error.contains(&format!("ui/{sibling}")), "{error}");
		assert!(error.contains("collides"), "{error}");
		assert!(!output.join("translations.rs").exists());
	}
}

#[test]
fn same_payload_alias_is_allowed_in_independent_groups() {
	let fixture = Fixture::new();
	let settings = fixture.catalogs();
	let output = fixture.0.join("target/generated");

	for language in ["de", "fr", "pt-BR"] {
		for group in ["left", "right"] {
			fixture.write(
				&format!("data/strings/languages/{language}/{group}/hud.ftl"),
				"# $icon (Element) - Inline icon.\nprompt = Press { $icon }\n",
			);
		}
	}

	generate(&fixture.0, &output, &settings).unwrap();
	let source = fs::read_to_string(output.join("translations.rs")).unwrap();

	assert_eq!(source.matches("Prompt as HudPrompt").count(), 2);
}

fn public_module_name(item: &Item) -> Option<String> {
	let Item::Mod(module) = item else {
		return None;
	};

	matches!(module.vis, Visibility::Public(_)).then(|| module.ident.to_string())
}
