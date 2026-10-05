//! Resolve references inside a module before upstream generation loses file scope.
use crate::{BuildError, schema::Schema};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Check same-file message/term targets and reject reference cycles.
/// Does not validate variable types or function availability; upstream separately
/// checks typed contracts.
pub(super) fn validate(schema: &Schema, path: &Path) -> Result<(), BuildError> {
	let mut edges = BTreeMap::new();

	for (key, references) in schema {
		let mut targets = Vec::new();

		for reference in references {
			let target = if let Some(message) = reference.strip_prefix("message:") {
				message.trim_end_matches('.').to_owned()
			} else if let Some(term) = reference.strip_prefix("term:") {
				format!("-{}", term.trim_end_matches('.'))
			} else {
				continue;
			};

			if !schema.contains_key(&target) {
				return Err(BuildError::UndefinedReference {
					path: path.into(),
					key: key.clone(),
					target,
				});
			}

			targets.push(target);
		}

		edges.insert(key.clone(), targets);
	}

	let mut visited = BTreeSet::new();
	let mut visiting = BTreeSet::new();

	for key in schema.keys() {
		visit(key, path, &edges, &mut visiting, &mut visited)?;
	}

	Ok(())
}

fn visit(
	key: &str,
	path: &Path,
	edges: &BTreeMap<String, Vec<String>>,
	visiting: &mut BTreeSet<String>,
	visited: &mut BTreeSet<String>,
) -> Result<(), BuildError> {
	if visited.contains(key) {
		return Ok(());
	}

	if !visiting.insert(key.to_owned()) {
		return Err(BuildError::CyclicReference {
			path: path.into(),
			key: key.into(),
		});
	}

	for target in &edges[key] {
		visit(target, path, edges, visiting, visited)?;
	}

	visiting.remove(key);
	visited.insert(key.to_owned());
	Ok(())
}
