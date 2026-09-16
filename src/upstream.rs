//! Invoke the public upstream generator once per module, without rewriting Fluent.
use crate::discovery::CatalogSources;
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
) -> Result<(), String> {
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
		fs::create_dir_all(&module_output).map_err(|error| error.to_string())?;

		for module in sources.modules.iter().filter(|module| &module.path == path) {
			let directory = module_input.join(&module.language);
			fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
			let input = directory.join("module.ftl");
			let bytes = fs::read(&module.absolute).map_err(|error| error.to_string())?;

			if fs::read(&input).ok().as_deref() != Some(bytes.as_slice()) {
				fs::write(input, bytes).map_err(|error| error.to_string())?;
			}
		}

		let options = BuildOptions::default()
			.with_locales_folder(
				module_input
					.to_str()
					.ok_or("module input path must be UTF-8")?,
			)
			// Upstream's default owns API annotations, not the app's startup language.
			.with_default_language(&sources.configuration.source_language)
			.with_lint_level(LintLevel::Strict)
			.with_output_file_path(
				module_output
					.join("translations.rs")
					.to_str()
					.ok_or("output path must be UTF-8")?,
			)
			.with_ftl_output(FtlOutputOptions::single_file(
				module_output
					.join("translations.ftl")
					.to_str()
					.ok_or("output path must be UTF-8")?,
			))
			.without_format();

		try_build_from_locales_folder(options)
			.map_err(|error| format!("module `{path}`: {error}"))?;
	}

	Ok(())
}
