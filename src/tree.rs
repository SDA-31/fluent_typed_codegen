//! Deterministic Rust namespaces derived from complete relative FTL paths.
use std::collections::BTreeMap;

#[derive(Default)]
/// Namespace node: the root has an empty path, folders have children, files a leaf index.
/// `path` preserves the extensionless source spelling; `name`/`ty` are normalized
/// Rust identifiers, with keyword names retaining their `r#` escape.
pub(super) struct Node {
	pub name: String,
	pub ty: String,
	pub path: String,
	pub leaf: Option<usize>,
	pub children: BTreeMap<String, Node>,
}

impl Node {
	pub fn build(paths: &[String]) -> Result<Self, String> {
		let mut root = Self {
			ty: "Translations".into(),
			..Self::default()
		};

		for (index, path) in paths.iter().enumerate() {
			let stem = path.strip_suffix(".ftl").ok_or("module must end in .ftl")?;
			let mut parent = &mut root;

			for (depth, part) in stem.split('/').enumerate() {
				let (name, ty) = names(part)?;

				if ty == "Locale"
					|| matches!(name.as_str(), "fluent_typed" | "fluent_syntax" | "std")
				{
					return Err(format!(
						"{path}: generated name `{ty}` or `{name}` is reserved by the catalog API"
					));
				}

				if depth == 0
					&& (ty == "Translations"
						|| matches!(name.as_str(), "locale" | "embedded" | "from_modules"))
				{
					return Err(format!(
						"{path}: generated name `{ty}` or `{name}` is reserved by the catalog API"
					));
				}

				if parent.leaf.is_some() {
					return Err(format!(
						"{path}: a module cannot be both a file and a directory ({})",
						parent.path
					));
				}

				if let Some(existing) = parent.children.values().find(|child| {
					(child.name == name || child.ty == ty)
						&& child.path.rsplit('/').next() != Some(part)
				}) {
					return Err(format!(
						"{path}: Rust namespace collision with `{}` after converting names to snake_case/PascalCase",
						existing.path
					));
				}

				let full = if parent.path.is_empty() {
					part.into()
				} else {
					format!("{}/{part}", parent.path)
				};
				parent = parent.children.entry(name.clone()).or_insert_with(|| Self {
					name,
					ty,
					path: full,
					..Self::default()
				});
			}

			if !parent.children.is_empty() || parent.leaf.is_some() {
				return Err(format!(
					"{path}: a module cannot be both a file and a directory ({})",
					parent.path
				));
			}

			parent.leaf = Some(index);
		}

		Ok(root)
	}
}

/// Convert one ASCII source-path component into snake_case and PascalCase names.
/// Reject invalid identifiers and keywords that cannot use a raw Rust identifier.
pub(super) fn names(source: &str) -> Result<(String, String), String> {
	if source.is_empty()
		|| !source
			.bytes()
			.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
	{
		return Err(format!(
			"invalid module/language name `{source}`: use ASCII letters, digits, underscores or hyphens"
		));
	}

	let characters: Vec<_> = source.chars().collect();
	let mut snake = String::new();

	for (index, &character) in characters.iter().enumerate() {
		if matches!(character, '_' | '-') {
			if !snake.is_empty() && !snake.ends_with('_') {
				snake.push('_');
			}

			continue;
		}

		let previous_lower = index > 0
			&& (characters[index - 1].is_ascii_lowercase()
				|| characters[index - 1].is_ascii_digit());
		let acronym_end = index > 0
			&& characters[index - 1].is_ascii_uppercase()
			&& characters
				.get(index + 1)
				.is_some_and(char::is_ascii_lowercase);

		if character.is_ascii_uppercase()
			&& (previous_lower || acronym_end)
			&& !snake.ends_with('_')
		{
			snake.push('_');
		}

		snake.push(character.to_ascii_lowercase());
	}

	let snake = snake.trim_end_matches('_').to_owned();
	let ty: String = snake
		.split('_')
		.filter(|part| !part.is_empty())
		.map(|part| {
			let mut result = part.to_owned();
			result[..1].make_ascii_uppercase();
			result
		})
		.collect();

	if ty.is_empty()
		|| !ty.as_bytes()[0].is_ascii_alphabetic()
		|| matches!(snake.as_str(), "self" | "super" | "crate")
	{
		return Err(format!(
			"`{source}` cannot identify a generated Rust module/type"
		));
	}

	// Raw identifiers preserve meaningful names such as `type.ftl` and `match.ftl`.
	let keyword =
		matches!(
			snake.as_str(),
			"as" | "async"
				| "await" | "break"
				| "const" | "continue"
				| "dyn" | "else"
				| "enum" | "extern"
				| "false" | "fn"
				| "for" | "if"
				| "impl" | "in"
				| "let" | "loop"
				| "match" | "mod"
				| "move" | "mut"
				| "pub" | "ref"
				| "return" | "static"
				| "struct" | "trait"
				| "true" | "type"
				| "unsafe" | "use"
				| "where" | "while"
				| "abstract" | "become"
				| "box" | "do"
				| "final" | "gen"
				| "macro" | "override"
				| "priv" | "try"
				| "typeof" | "unsized"
				| "virtual" | "yield"
		);
	Ok((if keyword { format!("r#{snake}") } else { snake }, ty))
}
