//! Generated contract checks: translations may change words, not their typed API.
use fluent_syntax::{ast, parser};
use std::{
	collections::{BTreeMap, BTreeSet},
	error::Error,
	fmt,
};

/// Fluent syntax, duplicate key or contract mismatch.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaError {
	/// Original parser failures with source byte ranges.
	Parse(Vec<parser::ParserError>),
	/// A message, term or attribute occurs more than once.
	DuplicateKey(String),
	/// Keys or referenced identifiers differ from the source contract.
	ChangedKeys(Vec<String>),
}

impl fmt::Display for SchemaError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Parse(errors) => write!(f, "{errors:?}"),
			Self::DuplicateKey(key) => write!(f, "duplicate Fluent key: {key}"),
			Self::ChangedKeys(keys) => write!(
				f,
				"Fluent keys/references changed; rebuild required: {keys:?}"
			),
		}
	}
}

impl Error for SchemaError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Parse(errors) => errors.first().map(|error| error as &(dyn Error + 'static)),
			_ => None,
		}
	}
}

/// Message/term/attribute keys mapped to their variable, message, term and function references.
///
/// This is a compatibility fingerprint, not a Rust argument-type checker. Typed
/// argument validation is also performed by the application's generated catalog.
pub type Schema = BTreeMap<String, BTreeSet<String>>;

/// Extract keys and references, ignoring translated prose and declaration order.
///
/// # Errors
/// Rejects Fluent syntax errors and duplicate keys.
pub fn schema(source: &str) -> Result<Schema, SchemaError> {
	let resource = parser::parse(source).map_err(|(_, errors)| SchemaError::Parse(errors))?;
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

fn insert(
	result: &mut Schema,
	key: String,
	pattern: &ast::Pattern<&str>,
) -> Result<(), SchemaError> {
	let mut refs = BTreeSet::new();
	visit_pattern(pattern, &mut refs);

	if result.insert(key.clone(), refs).is_some() {
		return Err(SchemaError::DuplicateKey(key));
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
pub fn validate(candidate: &str, expected: &str) -> Result<(), SchemaError> {
	let candidate = schema(candidate)?;
	let expected = schema(expected)?;

	compare(&candidate, &expected)
}

fn compare(candidate: &Schema, expected: &Schema) -> Result<(), SchemaError> {
	if candidate == expected {
		return Ok(());
	}

	let changed: Vec<_> = expected
		.keys()
		.chain(candidate.keys())
		.filter(|key| expected.get(*key) != candidate.get(*key))
		.cloned()
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect();

	Err(SchemaError::ChangedKeys(changed))
}

/// Check a candidate against a build-prepared key/reference fingerprint.
///
/// This form never needs the source translation's prose at runtime.
///
/// # Errors
/// Rejects invalid syntax, duplicate keys or changed keys/references.
// This function is copied into generated runtime code, rather than called by the host.
#[allow(dead_code)]
pub fn validate_schema(candidate: &str, expected: &[(&str, &[&str])]) -> Result<(), SchemaError> {
	let candidate = schema(candidate)?;
	let expected: Schema = expected
		.iter()
		.map(|(key, references)| {
			(
				(*key).to_owned(),
				references
					.iter()
					.map(|reference| (*reference).to_owned())
					.collect(),
			)
		})
		.collect();
	compare(&candidate, &expected)
}
