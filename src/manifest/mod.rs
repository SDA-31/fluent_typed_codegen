mod source;

pub use source::LocalizationManifest;

#[cfg(test)]
#[path = "tests/manifest.rs"]
mod tests;
