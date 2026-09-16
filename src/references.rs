//! Resolve references inside a module before upstream generation loses file scope.
use crate::schema::Schema;
use std::collections::{BTreeMap, BTreeSet};

/// Check same-file message/term targets and reject reference cycles.
/// Does not validate variable types or function availability; upstream separately
/// checks typed contracts.
pub(super) fn validate(schema: &Schema) -> Result<(), String> {
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
				return Err(format!(
					"`{key}` references `{target}`, which is not defined in this FTL module. Modules have independent namespaces; cross-file references require an explicit shared-resource policy."
				));
			}

			targets.push(target);
		}

		edges.insert(key.clone(), targets);
	}

	let mut visited = BTreeSet::new();
	let mut visiting = BTreeSet::new();

	for key in schema.keys() {
		visit(key, &edges, &mut visiting, &mut visited)?;
	}

	Ok(())
}

fn visit(
	key: &str,
	edges: &BTreeMap<String, Vec<String>>,
	visiting: &mut BTreeSet<String>,
	visited: &mut BTreeSet<String>,
) -> Result<(), String> {
	if visited.contains(key) {
		return Ok(());
	}

	if !visiting.insert(key.to_owned()) {
		return Err(format!("cyclic Fluent reference involving `{key}`"));
	}

	for target in &edges[key] {
		visit(target, edges, visiting, visited)?;
	}

	visiting.remove(key);
	visited.insert(key.to_owned());
	Ok(())
}
