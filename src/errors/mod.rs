//! Structured configuration, source and host-generation errors.
#[cfg(feature = "build")]
mod build;
mod config;
mod manifest;
#[cfg(feature = "build")]
mod schema;

#[cfg(feature = "build")]
pub use build::{
	BuildError, IoOperation, ModuleMismatch, NameError, NameOwner, SyntaxNode, UpstreamShapeError,
};
pub use config::{ConfigError, ConfigField, FieldError, PathError};
pub use manifest::ManifestError;
#[cfg(feature = "build")]
pub use schema::SchemaError;
