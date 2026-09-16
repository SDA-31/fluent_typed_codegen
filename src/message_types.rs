//! Public message payloads stay nameable without exposing upstream leaf modules.
use crate::tree::{Node, names};
use std::{collections::BTreeMap, fs, path::Path};
use syn::{Ident, Item, Visibility};

/// A name inside the private upstream leaf and its leaf-prefixed public alias.
pub(super) struct MessageType {
	pub original: Ident,
	pub alias: Ident,
}

pub(super) type MessageTypes = BTreeMap<String, Vec<MessageType>>;

/// Read type declarations from completed upstream output for the discovered FTL paths.
/// Infrastructure types remain private; payload aliases live in each leaf's parent.
pub(super) fn discover(paths: &[String], output: &Path) -> Result<MessageTypes, String> {
	let mut types = MessageTypes::new();

	for path in paths {
		let stem = path.strip_suffix(".ftl").expect("discovered module path");
		let source_path = output.join("modules").join(stem).join("translations.rs");
		let source = fs::read_to_string(&source_path)
			.map_err(|error| format!("{}: {error}", source_path.display()))?;
		let syntax = syn::parse_file(&source)
			.map_err(|error| format!("{}: {error}", source_path.display()))?;
		let leaf = stem.rsplit('/').next().expect("module has a name");
		let (_, prefix) = names(leaf)?;
		let mut exports = Vec::new();

		for item in syntax.items {
			let Some((original, visibility)) = public_type(item) else {
				continue;
			};

			// These two upstream infrastructure types back our Locale/leaf wrappers.
			if !matches!(visibility, Visibility::Public(_))
				|| matches!(original.to_string().as_str(), "L10n" | "L10nLanguage")
			{
				continue;
			}

			let name = original.to_string();
			let alias = format!("{prefix}{}", name.strip_prefix("r#").unwrap_or(&name));
			let alias = syn::parse_str(&alias)
				.map_err(|error| format!("{path}: invalid message type `{alias}`: {error}"))?;
			exports.push(MessageType { original, alias });
		}

		types.insert(stem.into(), exports);
	}

	Ok(types)
}

/// Reject aliases colliding with sibling wrappers or payloads in the same scope.
/// Identical aliases in independent directory namespaces remain valid.
pub(super) fn validate(node: &Node, types: &MessageTypes) -> Result<(), String> {
	let mut occupied: BTreeMap<_, _> = node
		.children
		.values()
		.map(|child| (child.ty.clone(), format!("catalog `{}`", child.path)))
		.collect();
	occupied.insert("Locale".into(), "generated Locale API".into());

	if node.path.is_empty() {
		occupied.insert("Translations".into(), "generated root API".into());
	}

	for child in node.children.values() {
		validate(child, types)?;
		let Some(exports) = types.get(&child.path) else {
			continue;
		};

		for export in exports {
			let alias = export.alias.to_string();
			let owner = format!("message type `{}` in `{}.ftl`", export.original, child.path);

			if let Some(previous) = occupied.insert(alias.clone(), owner.clone()) {
				return Err(format!(
					"generated message type `{alias}` for {owner} collides with {previous}; rename the module or message"
				));
			}
		}
	}

	Ok(())
}

fn public_type(item: Item) -> Option<(Ident, Visibility)> {
	match item {
		Item::Struct(item) => Some((item.ident, item.vis)),
		Item::Enum(item) => Some((item.ident, item.vis)),
		Item::Type(item) => Some((item.ident, item.vis)),
		Item::Union(item) => Some((item.ident, item.vis)),
		Item::Trait(item) => Some((item.ident, item.vis)),
		_ => None,
	}
}
