//! Typed host-generation failures; compiled only with the build feature.
use crate::{ConfigError, diagnostics};
use std::{
	error::Error,
	fmt, io,
	path::{Path, PathBuf},
};

/// Filesystem operation that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IoOperation {
	/// Reading a file.
	Read,
	/// Reading a directory or its next entry.
	ReadDirectory,
	/// Reading file type or metadata.
	Metadata,
	/// Writing an owned output or staged input.
	Write,
	/// Creating an output or staging directory.
	CreateDirectory,
	/// Resolving a filesystem path.
	Canonicalize,
}

impl fmt::Display for IoOperation {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Read => "read",
			Self::ReadDirectory => "read directory",
			Self::Metadata => "inspect metadata",
			Self::Write => "write",
			Self::CreateDirectory => "create directory",
			Self::Canonicalize => "canonicalize",
		})
	}
}

/// Why a source component cannot identify a Rust module or type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NameError {
	/// Only ASCII letters, digits, underscores and hyphens are allowed.
	InvalidCharacters,
	/// The normalized name is empty, starts with a digit, or is an unescapable keyword.
	InvalidIdentifier,
	/// A logical module path does not end in .ftl.
	ExpectedFtl,
}

/// Opaque Rust syntax category rejected before source printing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyntaxNode {
	/// An opaque Item node.
	Item,
	/// An opaque ForeignItem node.
	ForeignItem,
	/// An opaque ImplItem node.
	ImplItem,
	/// An opaque TraitItem node.
	TraitItem,
	/// An opaque Expr node.
	Expr,
	/// An opaque Pat node.
	Pat,
	/// An opaque Type node.
	Type,
	/// An opaque TypeParamBound node.
	TypeParamBound,
}

/// Unsupported structure in upstream generated source.
#[derive(Debug)]
#[non_exhaustive]
pub enum UpstreamShapeError {
	/// LANG_DATA is duplicated or is not an include_bytes expression.
	LangData,
	/// An unexpected upstream static remains.
	Static {
		/// Static name.
		name: String,
	},
	/// An unexpected upstream constant remains.
	Constant {
		/// Constant name.
		name: String,
	},
	/// L10n contains a non-function associated item.
	L10nItem,
	/// L10n contains an unrecognized method.
	L10nMethod {
		/// Method name.
		name: String,
	},
	/// Required embedded helpers or contracts are absent.
	MissingHelpers,
}

/// The generated declaration owning a conflicting Rust name.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NameOwner {
	/// A generated catalog wrapper, identified by its logical module path.
	Catalog(String),
	/// The generated locale selector.
	Locale,
	/// A generated loading helper type, identified by its Rust name.
	Loading(String),
	/// The generated root catalog.
	Root,
	/// A public upstream message payload.
	Message {
		/// Logical FTL module path.
		module: String,
		/// Original message type name.
		name: String,
	},
}

impl fmt::Display for NameOwner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Catalog(path) => write!(f, "catalog `{path}`"),
			Self::Locale => f.write_str("generated Locale API"),
			Self::Loading(_) => f.write_str("generated loading API"),
			Self::Root => f.write_str("generated root API"),
			Self::Message { module, name } => write!(f, "message type `{name}` in `{module}`"),
		}
	}
}

/// Per-language difference from the complete source module inventory.
#[derive(Debug)]
pub struct ModuleMismatch {
	/// Translation language.
	pub locale: String,
	/// Language defining the module inventory.
	pub source_language: String,
	/// Translation language directory.
	pub locale_root: PathBuf,
	/// Source language directory.
	pub reference_root: PathBuf,
	/// Missing logical module paths, sorted.
	pub missing: Vec<String>,
	/// Extra logical module paths, sorted.
	pub extra: Vec<String>,
}

impl fmt::Display for ModuleMismatch {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&diagnostics::module_mismatch(
			&self.locale,
			&self.source_language,
			&self.locale_root,
			&self.reference_root,
			&self.extra,
			&self.missing,
		))
	}
}

/// Configuration, catalog validation, generation or filesystem failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum BuildError {
	/// A required Cargo environment variable is absent.
	Environment {
		/// Missing environment variable.
		variable: &'static str,
	},
	/// A filesystem operation failed.
	Io {
		/// Attempted operation.
		operation: IoOperation,
		/// Affected filesystem path.
		path: PathBuf,
		/// Original I/O failure.
		source: io::Error,
	},
	/// Cargo metadata or localization TOML is invalid.
	Config {
		/// Configuration file, when known.
		path: Option<PathBuf>,
		/// Typed configuration failure.
		source: ConfigError,
	},
	/// A filesystem path cannot be embedded in generated Rust.
	NonUtf8Path {
		/// Original non-UTF-8 path.
		path: PathBuf,
	},
	/// A symlink occurs inside the catalog tree.
	Symlink {
		/// Rejected symlink.
		path: PathBuf,
	},
	/// A language directory has an invalid locale identifier.
	InvalidLocale {
		/// Language directory.
		path: PathBuf,
		/// Original directory spelling.
		locale: String,
		/// Original locale parser error.
		source: <fluent_typed::prelude::LanguageIdentifier as std::str::FromStr>::Err,
	},
	/// A locale directory does not use canonical spelling.
	NonCanonicalLocale {
		/// Original language directory.
		path: PathBuf,
		/// Original directory spelling.
		locale: String,
		/// Required canonical spelling.
		canonical: String,
	},
	/// A configured locale has no directory.
	MissingLocale {
		/// Configured locale.
		locale: String,
		/// Language directory root.
		root: PathBuf,
	},
	/// The source language has no FTL modules.
	EmptyCatalog {
		/// Source language directory.
		root: PathBuf,
		/// Source language.
		locale: String,
	},
	/// Translations do not have the source language module set.
	ModuleMismatch(Box<ModuleMismatch>),
	/// A module has invalid Fluent syntax or contracts.
	Schema {
		/// FTL input file.
		path: PathBuf,
		/// Input language.
		locale: String,
		/// Typed Fluent schema failure.
		source: crate::SchemaError,
	},
	/// A message refers to a target outside its module namespace.
	UndefinedReference {
		/// FTL input file.
		path: PathBuf,
		/// Referring message, term or attribute.
		key: String,
		/// Missing reference target.
		target: String,
	},
	/// A module has a cyclic Fluent reference.
	CyclicReference {
		/// FTL input file.
		path: PathBuf,
		/// Key encountered again in the active traversal.
		key: String,
	},
	/// A source component cannot become a Rust name.
	InvalidName {
		/// Original name or path.
		name: String,
		/// Machine-readable reason.
		reason: NameError,
	},
	/// A generated name is reserved by the catalog or extension.
	ReservedName {
		/// Logical module path.
		path: String,
		/// Generated module name.
		name: String,
		/// Generated catalog type name.
		ty: String,
		/// Whether an extension reserves the name.
		extension: bool,
	},
	/// Two module paths normalize to the same Rust name.
	NamespaceCollision {
		/// Conflicting logical module path.
		path: String,
		/// Earlier logical module path.
		other: String,
	},
	/// A path is used as both a leaf and a group.
	FileDirectoryCollision {
		/// FTL path being added.
		path: String,
		/// Conflicting namespace node.
		node: String,
	},
	/// Two locale codes normalize to the same enum variant.
	LocaleCollision {
		/// First locale code.
		first: String,
		/// Second locale code.
		second: String,
		/// Generated enum variant.
		variant: String,
	},
	/// A public message payload conflicts with another generated name.
	MessageTypeCollision {
		/// Generated message alias.
		alias: String,
		/// Declaration owning the conflicting message.
		owner: NameOwner,
		/// Earlier declaration owning the same name.
		previous: NameOwner,
	},
	/// Generated Rust or an upstream output cannot be parsed.
	Syntax {
		/// Generated filename or path.
		file: PathBuf,
		/// Original Rust syntax error.
		source: syn::Error,
	},
	/// An upstream message cannot become a public Rust alias.
	InvalidMessageType {
		/// Logical FTL module path.
		module: String,
		/// Requested alias.
		alias: String,
		/// Original identifier parser error.
		source: syn::Error,
	},
	/// A syntax extension emitted an opaque Verbatim node.
	UnsupportedSyntax {
		/// Generated filename.
		file: PathBuf,
		/// Opaque syntax node category.
		node: SyntaxNode,
	},
	/// An extension filename is unsafe or conflicts with owned output.
	InvalidExtensionFilename {
		/// Requested extension filename.
		filename: String,
	},
	/// The upstream typed Fluent generator rejected a module.
	Upstream {
		/// Logical FTL module path.
		module: String,
		/// Original upstream build error.
		source: Box<fluent_typed::BuildError>,
	},
	/// Upstream generated an unsupported source structure.
	UpstreamShape {
		/// Upstream generated Rust file.
		path: PathBuf,
		/// Unsupported structure.
		reason: UpstreamShapeError,
	},
	/// Discovery produced a path outside its source root.
	PathOutsideRoot {
		/// Discovered path.
		path: PathBuf,
		/// Expected source root.
		root: PathBuf,
		/// Original path-prefix error.
		source: std::path::StripPrefixError,
	},
}

impl BuildError {
	pub(crate) fn io(operation: IoOperation, path: &Path, source: io::Error) -> Self {
		Self::Io {
			operation,
			path: path.into(),
			source,
		}
	}

	pub(crate) fn utf8(path: &Path) -> Result<&str, Self> {
		path.to_str()
			.ok_or_else(|| Self::NonUtf8Path { path: path.into() })
	}
}

impl From<ConfigError> for BuildError {
	fn from(source: ConfigError) -> Self {
		Self::Config { path: None, source }
	}
}

impl fmt::Display for BuildError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Environment { variable } => write!(f, "{variable} is unset"),
			Self::Io {
				operation,
				path,
				source,
			} => write!(f, "{operation} {}: {source}", path.display()),
			Self::Config { path, source } => {
				if let Some(path) = path {
					write!(f, "{}: ", path.display())?;
				}

				source.fmt(f)
			}
			Self::NonUtf8Path { path } => write!(f, "path must be UTF-8: {}", path.display()),
			Self::Symlink { path } => write!(
				f,
				"symbolic links inside catalogs are unsupported: {}",
				path.display()
			),
			Self::InvalidLocale { locale, source, .. } => {
				write!(f, "invalid language directory {locale:?}: {source}")
			}
			Self::NonCanonicalLocale {
				locale, canonical, ..
			} => write!(
				f,
				"language directory {locale:?} must use canonical spelling {canonical:?}"
			),
			Self::MissingLocale { locale, root } => write!(
				f,
				"configured language {locale:?} has no directory in {}",
				root.display()
			),
			Self::EmptyCatalog { locale, .. } => {
				write!(f, "no .ftl modules in source language {locale}")
			}
			Self::ModuleMismatch(details) => details.fmt(f),
			Self::Schema { path, source, .. } => write!(f, "{}: {source}", path.display()),
			Self::UndefinedReference { path, key, target } => write!(
				f,
				"{}: `{key}` references `{target}`, which is not defined in this FTL module. Modules have independent namespaces; cross-file references require an explicit shared-resource policy.",
				path.display()
			),
			Self::CyclicReference { path, key } => write!(
				f,
				"{}: cyclic Fluent reference involving `{key}`",
				path.display()
			),
			Self::InvalidName { name, reason } => match reason {
				NameError::InvalidCharacters => write!(
					f,
					"invalid module/language name `{name}`: use ASCII letters, digits, underscores or hyphens"
				),
				NameError::InvalidIdentifier => {
					write!(f, "`{name}` cannot identify a generated Rust module/type")
				}
				NameError::ExpectedFtl => write!(f, "module must end in .ftl: {name}"),
			},
			Self::ReservedName {
				path,
				name,
				ty,
				extension,
			} => {
				if *extension {
					write!(f, "{path}: name `{name}` is reserved by the extension")
				} else {
					write!(
						f,
						"{path}: generated name `{ty}` or `{name}` is reserved by the catalog API"
					)
				}
			}
			Self::NamespaceCollision { path, other } => write!(
				f,
				"{path}: Rust namespace collision with `{other}` after converting names to snake_case/PascalCase"
			),
			Self::FileDirectoryCollision { path, node } => write!(
				f,
				"{path}: a module cannot be both a file and a directory ({node})"
			),
			Self::LocaleCollision {
				first,
				second,
				variant,
			} => write!(
				f,
				"locale names `{first}` and `{second}` collide as Rust variant `{variant}`"
			),
			Self::MessageTypeCollision {
				alias,
				owner,
				previous,
			} => write!(
				f,
				"generated message type `{alias}` for {owner} collides with {previous}; rename the module or message"
			),
			Self::Syntax { file, source } => {
				write!(f, "generated Rust `{}`: {source}", file.display())
			}
			Self::InvalidMessageType {
				module,
				alias,
				source,
			} => write!(f, "{module}: invalid message type `{alias}`: {source}"),
			Self::UnsupportedSyntax { file, node } => write!(
				f,
				"generated Rust `{}`: unsupported opaque {node:?} (Verbatim); emit structured Rust syntax instead",
				file.display()
			),
			Self::InvalidExtensionFilename { filename } => {
				write!(f, "invalid extension output filename: {filename:?}")
			}
			Self::Upstream { module, source } => write!(f, "module `{module}`: {source}"),
			Self::UpstreamShape { path, reason } => {
				write!(f, "{}: unsupported upstream ", path.display())?;

				match reason {
					UpstreamShapeError::LangData => f.write_str("LANG_DATA shape"),
					UpstreamShapeError::Static { name } => write!(f, "static: {name}"),
					UpstreamShapeError::Constant { name } => write!(f, "constant: {name}"),
					UpstreamShapeError::L10nItem => f.write_str("L10n item"),
					UpstreamShapeError::L10nMethod { name } => write!(f, "L10n method: {name}"),
					UpstreamShapeError::MissingHelpers => {
						f.write_str("generation shape: missing embedded helpers/contracts")
					}
				}
			}
			Self::PathOutsideRoot { path, root, source } => write!(
				f,
				"{} is outside {}: {source}",
				path.display(),
				root.display()
			),
		}
	}
}

impl Error for BuildError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Io { source, .. } => Some(source),
			Self::Config { source, .. } => Some(source),
			Self::InvalidLocale { source, .. } => Some(source),
			Self::Schema { source, .. } => Some(source),
			Self::Syntax { source, .. } | Self::InvalidMessageType { source, .. } => Some(source),
			Self::Upstream { source, .. } => Some(source.as_ref()),
			Self::PathOutsideRoot { source, .. } => Some(source),
			_ => None,
		}
	}
}
