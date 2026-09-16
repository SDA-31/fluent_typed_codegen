//! Explicit generation for a custom frontend; normally use from_cargo in build.rs.
use fluent_typed_codegen::{Settings, generate};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), String> {
	let mut args = env::args_os().skip(1);
	let (Some(package), Some(output)) = (args.next(), args.next()) else {
		return Err("usage: generate <consumer-package> <target-output-directory>".into());
	};

	if args.next().is_some() {
		return Err("expected exactly two paths".into());
	}

	let package = PathBuf::from(package);
	let output = PathBuf::from(output);
	let manifest =
		fs::read_to_string(package.join("Cargo.toml")).map_err(|error| error.to_string())?;
	let settings = Settings::from_manifest(&manifest)?;
	generate(&package, &output, &settings)?;
	println!("Generated localization sources in {}", output.display());
	Ok(())
}
