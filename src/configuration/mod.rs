mod catalog;

pub use catalog::CatalogConfig;
#[cfg(feature = "build")]
pub(crate) use catalog::string_field;

#[cfg(all(test, feature = "build"))]
#[path = "tests/configuration.rs"]
mod tests;
