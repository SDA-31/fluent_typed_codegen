//! Generated contract checks: translations may change words, not their typed API.
use fluent_syntax::{ast, parser};
use std::collections::{BTreeMap, BTreeSet};

/// Message/term/attribute keys mapped to their variable, message, term and function references.
///
/// This is a compatibility fingerprint, not a Rust argument-type checker. Typed
/// argument validation is also performed by the application's generated catalog.
pub type Schema = BTreeMap<String, BTreeSet<String>>;

/// Extract keys and references, ignoring translated prose and declaration order.
///
/// # Errors
/// Rejects Fluent syntax errors and duplicate keys.
pub fn schema(source: &str) -> Result<Schema, String> {
	let resource = parser::parse(source).map_err(|(_, errors)| format!("{errors:?}"))?;
	let mut result = Schema::new();

	for entry in resource.body {
		match entry {
			ast::Entry::Message(message) => {
				if let Some(value) = message.value {
					insert(&mut result, message.id.name.into(), &value)?;
				}

				for attribute in message.attributes {
					insert(
						&mut result,
						format!("{}.{}", message.id.name, attribute.id.name),
						&attribute.value,
					)?;
				}
			}
			ast::Entry::Term(term) => {
				insert(&mut result, format!("-{}", term.id.name), &term.value)?;

				for attribute in term.attributes {
					insert(
						&mut result,
						format!("-{}.{}", term.id.name, attribute.id.name),
						&attribute.value,
					)?;
				}
			}
			_ => {}
		}
	}

	Ok(result)
}

fn insert(result: &mut Schema, key: String, pattern: &ast::Pattern<&str>) -> Result<(), String> {
	let mut refs = BTreeSet::new();
	visit_pattern(pattern, &mut refs);

	if result.insert(key.clone(), refs).is_some() {
		return Err(format!("duplicate Fluent key: {key}"));
	}

	Ok(())
}

fn visit_pattern(pattern: &ast::Pattern<&str>, refs: &mut BTreeSet<String>) {
	for element in &pattern.elements {
		let ast::PatternElement::Placeable { expression } = element else {
			continue;
		};

		visit_expression(expression, refs);
	}
}

fn visit_expression(expression: &ast::Expression<&str>, refs: &mut BTreeSet<String>) {
	match expression {
		ast::Expression::Inline(inline) => visit_inline(inline, refs),
		ast::Expression::Select { selector, variants } => {
			visit_inline(selector, refs);

			for variant in variants {
				visit_pattern(&variant.value, refs);
			}
		}
	}
}

fn visit_inline(inline: &ast::InlineExpression<&str>, refs: &mut BTreeSet<String>) {
	match inline {
		ast::InlineExpression::VariableReference { id } => {
			refs.insert(format!("variable:{}", id.name));
		}
		ast::InlineExpression::MessageReference { id, attribute } => {
			refs.insert(format!(
				"message:{}.{}",
				id.name,
				attribute.as_ref().map_or("", |a| a.name)
			));
		}
		ast::InlineExpression::TermReference {
			id,
			attribute,
			arguments,
		} => {
			refs.insert(format!(
				"term:{}.{}",
				id.name,
				attribute.as_ref().map_or("", |a| a.name)
			));

			if let Some(arguments) = arguments {
				visit_arguments(arguments, refs);
			}
		}
		ast::InlineExpression::FunctionReference { id, arguments } => {
			refs.insert(format!("function:{}", id.name));
			visit_arguments(arguments, refs);
		}
		ast::InlineExpression::Placeable { expression } => visit_expression(expression, refs),
		ast::InlineExpression::StringLiteral { .. }
		| ast::InlineExpression::NumberLiteral { .. } => {}
	}
}

fn visit_arguments(arguments: &ast::CallArguments<&str>, refs: &mut BTreeSet<String>) {
	for argument in &arguments.positional {
		visit_inline(argument, refs);
	}

	for argument in &arguments.named {
		visit_inline(&argument.value, refs);
	}
}

/// Check that a translation preserves every key and its referenced identifiers.
///
/// # Errors
/// Rejects invalid syntax, duplicate keys or a different compatibility fingerprint.
///
/// ```ignore
/// assert!(validate("hello = Hola { $name }", "hello = Hello { $name }").is_ok());
/// assert!(validate("hello = Hi { $who }", "hello = Hello { $name }").is_err());
/// ```
pub fn validate(candidate: &str, expected: &str) -> Result<(), String> {
	let candidate = schema(candidate)?;
	let expected = schema(expected)?;

	if candidate == expected {
		return Ok(());
	}

	let changed: Vec<_> = expected
		.keys()
		.chain(candidate.keys())
		.filter(|key| expected.get(*key) != candidate.get(*key))
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect();

	Err(format!(
		"Fluent keys/references changed; rebuild required: {changed:?}"
	))
}
