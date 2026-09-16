//! Opt-in Rust syntax decoration for integrations, without runtime dependencies.
use syn::{Attribute, Ident, Item, ItemUse, Path};

/// A generated type and its path of accessor calls from the root snapshot.
///
/// Describes root, group and leaf wrappers only, not locales or message payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
	/// Rust type path relative to the generated root, e.g. `presentation::Hud`.
	pub type_path: Path,
	/// Rust method names, e.g. `["presentation", "hud"]`; empty for the root.
	pub accessors: Vec<Ident>,
}

/// Syntax-level extension for a consuming framework's generated entrypoint.
///
/// The generator always emits its plain `translations.rs`. An extension adds one
/// independently rendered facade sharing the same upstream modules and contracts.
/// Hooks return Rust syntax nodes, not source strings or user-supplied FTL.
/// Use the re-exported [`syn::parse_quote!`] to construct them. Hooks must not
/// perform I/O. The generated file is syntax-checked before writing; type/name
/// resolution still happens when the consuming crate compiles it.
/// Opaque `Verbatim` syntax nodes are rejected; macro bodies remain opaque tokens.
/// Panics in hooks are not caught or converted into generation diagnostics.
/// Resource registration, framework traits and dependency aliases belong here,
/// not in the generator. The host must arrange the dependencies named by its hooks.
///
/// ```
/// use fluent_typed_codegen::{Extension, Scope, syn::{Attribute, Item, parse_quote}};
///
/// struct Adapter;
///
/// impl Extension for Adapter {
///     fn filename(&self) -> &str {
///         "adapter.rs"
///     }
///
///     fn type_attributes(&self) -> Vec<Attribute> {
///         vec![parse_quote!(#[allow(dead_code)])]
///     }
///
///     fn root_items(&self, scopes: &[Scope]) -> Vec<Item> {
///         let count = scopes.len();
///
///         vec![parse_quote!(pub const SCOPE_COUNT: usize = #count;)]
///     }
/// }
/// ```
///
/// Source strings cannot be passed in place of attributes:
///
/// ```compile_fail
/// use fluent_typed_codegen::syn::Attribute;
///
/// fn attributes() -> Vec<Attribute> {
///     vec!["#[allow(dead_code)]".into()]
/// }
/// ```
pub trait Extension {
	/// Additional `.rs` filename directly inside the output directory.
	/// Must not contain a path or use `translations.rs`, `locale_modules.rs` or
	/// `validation.rs`, which belong to the generator.
	fn filename(&self) -> &str;

	/// Use declarations before the tree at its root.
	fn root_imports(&self) -> Vec<ItemUse> {
		Vec::new()
	}

	/// Imports inside every nested directory namespace (not the upstream leaf).
	fn scope_imports(&self) -> Vec<ItemUse> {
		Vec::new()
	}

	/// Attributes attached to every root, group and leaf wrapper.
	fn type_attributes(&self) -> Vec<Attribute> {
		Vec::new()
	}

	/// Extra normalized snake_case branch and leaf names forbidden at any depth.
	/// Supply keyword names without the `r#` prefix, e.g. `type`.
	fn reserved_names(&self) -> &[&str] {
		&[]
	}

	/// Items appended at the root, with root-first depth-first wrapper descriptors.
	/// Siblings are ordered by normalized Rust identifiers; locale and message
	/// payload types are excluded from `scopes`.
	fn root_items(&self, scopes: &[Scope]) -> Vec<Item>;
}
