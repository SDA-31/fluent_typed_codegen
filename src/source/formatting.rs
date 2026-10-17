//! Validate and print generated Rust; no formatter process or source-tree edits.
use crate::{BuildError, SyntaxNode};
use proc_macro2::TokenStream;
use syn::visit::{self, Visit};

pub(crate) fn format(tokens: TokenStream, filename: &str) -> Result<String, BuildError> {
	let file = syn::parse2(tokens).map_err(|source| BuildError::Syntax {
		file: filename.into(),
		source,
	})?;
	let mut syntax = StructuredSyntax::default();
	syntax.visit_file(&file);

	if let Some(kind) = syntax.unsupported {
		return Err(BuildError::UnsupportedSyntax {
			file: filename.into(),
			node: kind,
		});
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
	unsupported: Option<SyntaxNode>,
}

impl<'ast> Visit<'ast> for StructuredSyntax {
	fn visit_item(&mut self, node: &'ast syn::Item) {
		if matches!(node, syn::Item::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::Item);

			return;
		}

		visit::visit_item(self, node);
	}

	fn visit_foreign_item(&mut self, node: &'ast syn::ForeignItem) {
		if matches!(node, syn::ForeignItem::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::ForeignItem);

			return;
		}

		visit::visit_foreign_item(self, node);
	}

	fn visit_impl_item(&mut self, node: &'ast syn::ImplItem) {
		if matches!(node, syn::ImplItem::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::ImplItem);

			return;
		}

		visit::visit_impl_item(self, node);
	}

	fn visit_trait_item(&mut self, node: &'ast syn::TraitItem) {
		if matches!(node, syn::TraitItem::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::TraitItem);

			return;
		}

		visit::visit_trait_item(self, node);
	}

	fn visit_expr(&mut self, node: &'ast syn::Expr) {
		if matches!(node, syn::Expr::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::Expr);

			return;
		}

		visit::visit_expr(self, node);
	}

	fn visit_pat(&mut self, node: &'ast syn::Pat) {
		if matches!(node, syn::Pat::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::Pat);

			return;
		}

		visit::visit_pat(self, node);
	}

	fn visit_type(&mut self, node: &'ast syn::Type) {
		if matches!(node, syn::Type::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::Type);

			return;
		}

		visit::visit_type(self, node);
	}

	fn visit_type_param_bound(&mut self, node: &'ast syn::TypeParamBound) {
		if matches!(node, syn::TypeParamBound::Verbatim(_)) {
			self.unsupported.get_or_insert(SyntaxNode::TypeParamBound);

			return;
		}

		visit::visit_type_param_bound(self, node);
	}
}

#[cfg(test)]
#[path = "tests/formatting.rs"]
mod tests;
