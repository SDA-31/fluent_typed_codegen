//! Fluent schema validation failures shared by host and generated runtime.
use fluent_syntax::parser::ParserError;
use std::{error::Error, fmt};

/// Fluent syntax, duplicate key or contract mismatch.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaError {
	/// Original parser failures with source byte ranges.
	Parse(Vec<ParserError>),
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
