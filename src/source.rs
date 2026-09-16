//! Validate and print generated Rust; no formatter process or source-tree edits.
use proc_macro2::TokenStream;
use syn::visit::{self, Visit};

pub(super) fn format(tokens: TokenStream, filename: &str) -> Result<String, String> {
	let file =
		syn::parse2(tokens).map_err(|error| format!("generated Rust `{filename}`: {error}"))?;
	let mut syntax = StructuredSyntax::default();
	syntax.visit_file(&file);

	if let Some(kind) = syntax.unsupported {
		return Err(format!(
			"generated Rust `{filename}`: unsupported opaque {kind} (Verbatim); emit structured Rust syntax instead"
		));
	}

	Ok(format!(
		"// Generated localization source. Edit the original catalogs, not this file.\n{}",
		prettyplease::unparse(&file),
	))
}

// Prettyplease does not support Syn's opaque fallback nodes by default.
// Check recursively instead of catching a panic (whose hook would still print).
// Macro token bodies remain opaque deliberately: their grammar belongs to Rust.
#[derive(Default)]
struct StructuredSyntax {
	unsupported: Option<&'static str>,
}

impl<'ast> Visit<'ast> for StructuredSyntax {
	fn visit_item(&mut self, node: &'ast syn::Item) {
		if matches!(node, syn::Item::Verbatim(_)) {
			self.unsupported.get_or_insert("Item");

			return;
		}

		visit::visit_item(self, node);
	}

	fn visit_foreign_item(&mut self, node: &'ast syn::ForeignItem) {
		if matches!(node, syn::ForeignItem::Verbatim(_)) {
			self.unsupported.get_or_insert("ForeignItem");

			return;
		}

		visit::visit_foreign_item(self, node);
	}

	fn visit_impl_item(&mut self, node: &'ast syn::ImplItem) {
		if matches!(node, syn::ImplItem::Verbatim(_)) {
			self.unsupported.get_or_insert("ImplItem");

			return;
		}

		visit::visit_impl_item(self, node);
	}

	fn visit_trait_item(&mut self, node: &'ast syn::TraitItem) {
		if matches!(node, syn::TraitItem::Verbatim(_)) {
			self.unsupported.get_or_insert("TraitItem");

			return;
		}

		visit::visit_trait_item(self, node);
	}

	fn visit_expr(&mut self, node: &'ast syn::Expr) {
		if matches!(node, syn::Expr::Verbatim(_)) {
			self.unsupported.get_or_insert("Expr");

			return;
		}

		visit::visit_expr(self, node);
	}

	fn visit_pat(&mut self, node: &'ast syn::Pat) {
		if matches!(node, syn::Pat::Verbatim(_)) {
			self.unsupported.get_or_insert("Pat");

			return;
		}

		visit::visit_pat(self, node);
	}

	fn visit_type(&mut self, node: &'ast syn::Type) {
		if matches!(node, syn::Type::Verbatim(_)) {
			self.unsupported.get_or_insert("Type");

			return;
		}

		visit::visit_type(self, node);
	}

	fn visit_type_param_bound(&mut self, node: &'ast syn::TypeParamBound) {
		if matches!(node, syn::TypeParamBound::Verbatim(_)) {
			self.unsupported.get_or_insert("TypeParamBound");

			return;
		}

		visit::visit_type_param_bound(self, node);
	}
}

#[cfg(test)]
mod tests {
	use super::format;
	use quote::quote;

	#[test]
	fn opaque_syntax_is_a_diagnostic_including_inside_nested_scopes() {
		for tokens in [
			quote!(
				fn declared();
			),
			quote!(
				mod nested {
					fn declared();
				}
			),
			quote!(
				fn outer() {
					fn declared();
				}
			),
			quote!(
				use {::std::fmt};
			),
		] {
			let error = format(tokens, "adapter.rs").unwrap_err();

			assert!(error.contains("generated Rust `adapter.rs`"), "{error}");
			assert!(error.contains("Verbatim"), "{error}");
		}
	}

	#[test]
	fn macro_tokens_are_not_mistaken_for_unsupported_rust_syntax() {
		let output = format(
			quote! {
				macro_rules! custom {
					($value:expr) => { $value };
				}

				fn value() -> u8 {
					custom!(7)
				}
			},
			"adapter.rs",
		)
		.unwrap();

		assert!(output.contains("fn value() -> u8"));
		assert!(syn::parse_file(&output).is_ok());
	}
}
