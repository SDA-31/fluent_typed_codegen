//! Invoke the public upstream generator once per module, without rewriting Fluent.
use crate::{BuildError, UpstreamShapeError, build_io, discovery::CatalogSources};
use fluent_typed::{BuildOptions, FtlOutputOptions, LintLevel, try_build_from_locales_folder};
use std::{
	collections::hash_map::DefaultHasher,
	fs,
	hash::{Hash, Hasher},
	path::Path,
};

pub(super) fn generate(
	sources: &CatalogSources,
	paths: &[String],
	output: &Path,
) -> Result<(), BuildError> {
	// Upstream emits Cargo rerun dependencies on these inputs. Keep them under
	// target instead of deleting them and causing a perpetual rebuild. A changed
	// module/language inventory selects a fresh tree, so removed locales cannot leak.
	let mut topology = DefaultHasher::new();

	for module in &sources.modules {
		(&module.language, &module.path).hash(&mut topology);
	}

	let inputs = output
		.join("inputs")
		.join(format!("{:016x}", topology.finish()));

	for path in paths {
		let stem = path.strip_suffix(".ftl").expect("discovered FTL path");
		let module_input = inputs.join(stem);
		let module_output = output.join("modules").join(stem);
		build_io::create_dir_all(&module_output)?;

		for module in sources.modules.iter().filter(|module| &module.path == path) {
			let directory = module_input.join(&module.language);
			build_io::create_dir_all(&directory)?;
			let input = directory.join("module.ftl");
			let bytes = build_io::read(&module.absolute)?;

			if fs::read(&input).ok().as_deref() != Some(bytes.as_slice()) {
				build_io::write(input, bytes)?;
			}
		}

		let options = BuildOptions::default()
			.with_locales_folder(BuildError::utf8(&module_input)?)
			// Upstream's default owns API annotations, not the app's startup language.
			.with_default_language(&sources.configuration.source_language)
			.with_lint_level(LintLevel::Strict)
			.with_output_file_path(BuildError::utf8(&module_output.join("translations.rs"))?)
			.with_ftl_output(FtlOutputOptions::single_file(BuildError::utf8(
				&module_output.join("translations.ftl"),
			)?))
			.without_format();

		try_build_from_locales_folder(options).map_err(|source| BuildError::Upstream {
			module: path.clone(),
			source: Box::new(source),
		})?;
		remove_embedded(&module_output.join("translations.rs"))?;
	}

	Ok(())
}

/// Remove upstream's storage policy while preserving typed message accessors.
/// Reject an unfamiliar generator shape instead of silently shipping FTL payloads.
fn remove_embedded(output: &Path) -> Result<(), BuildError> {
	use syn::{ImplItem, Item, Type, parse_quote};

	let text = build_io::read_to_string(output)?;
	let mut syntax = syn::parse_file(&text).map_err(|source| BuildError::Syntax {
		file: output.into(),
		source,
	})?;
	let mut data = false;
	let mut contracts = false;
	let mut removed = std::collections::BTreeSet::new();
	let mut items = Vec::new();

	for mut item in syntax.items {
		match &mut item {
			Item::Static(value) if value.ident == "LANG_DATA" => {
				if data
					|| !matches!(value.expr.as_ref(), syn::Expr::Macro(value) if value.mac.path.is_ident("include_bytes"))
				{
					return Err(BuildError::UpstreamShape {
						path: output.into(),
						reason: UpstreamShapeError::LangData,
					});
				}

				data = true;
				continue;
			}
			Item::Static(value) if value.ident == "MESSAGE_CONTRACTS" => {
				contracts = true;
			}
			Item::Static(value)
				if value.ident != "ALL_LANGS"
					&& !matches!(value.ty.as_ref(), Type::Path(ty) if ty.path.is_ident("LanguageIdentifier")) =>
			{
				return Err(BuildError::UpstreamShape {
					path: output.into(),
					reason: UpstreamShapeError::Static {
						name: value.ident.to_string(),
					},
				});
			}
			Item::Const(value) => {
				return Err(BuildError::UpstreamShape {
					path: output.into(),
					reason: UpstreamShapeError::Constant {
						name: value.ident.to_string(),
					},
				});
			}
			Item::Use(import) if matches!(&import.tree, syn::UseTree::Path(path) if path.ident == "std") =>
			{
				item = parse_quote! {
					use std::{fmt::Display, ops::Deref, slice::Iter, str::FromStr};
				};
			}
			Item::Impl(implementation)
				if implementation.trait_.is_none()
					&& matches!(implementation.self_ty.as_ref(), Type::Path(ty) if ty.path.is_ident("L10n")) =>
			{
				let mut retained = Vec::new();

				for item in std::mem::take(&mut implementation.items) {
					let ImplItem::Fn(function) = &item else {
						return Err(BuildError::UpstreamShape {
							path: output.into(),
							reason: UpstreamShapeError::L10nItem,
						});
					};
					let name = function.sig.ident.to_string();

					match name.as_str() {
						"byte_range" | "load" | "load_all" | "language_name" => {
							removed.insert(name);
						}
						"iter" | "langneg" => retained.push(item),
						_ => {
							return Err(BuildError::UpstreamShape {
								path: output.into(),
								reason: UpstreamShapeError::L10nMethod { name },
							});
						}
					}
				}

				implementation.items = retained;
			}
			_ => {}
		}

		items.push(item);
	}

	if !data
		|| !contracts
		|| !["byte_range", "load", "load_all"]
			.iter()
			.all(|name| removed.contains(*name))
	{
		return Err(BuildError::UpstreamShape {
			path: output.into(),
			reason: UpstreamShapeError::MissingHelpers,
		});
	}

	items.push(parse_quote! {
		pub(super) fn __validate(bytes: &[u8]) -> Result<(), L10nError> {
			validate_ftl(bytes, MESSAGE_CONTRACTS)
		}
	});
	syntax.items = items;
	build_io::write(output, prettyplease::unparse(&syntax))
}
