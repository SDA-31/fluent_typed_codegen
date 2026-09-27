/// Declare a module containing this package's generated Fluent translations.
///
/// Run `fluent_typed_codegen::build()` from your build.rs with feature `build`.
/// In normal dependencies, disable default features to use just this macro.
/// Also declare `fluent-typed` and `fluent-syntax` under their standard dependency
/// names; generated code uses these consumer-owned runtime libraries.
///
/// ```ignore
/// fluent_typed_codegen::translations!(pub mod texts);
///
/// let manifest = texts::embed_manifest!();
/// let translations = texts::Translations::from_manifest(texts::Locale::En, &manifest)?;
/// ```
///
/// This example needs application-owned FTL and build output; the
/// [runnable example](https://github.com/SDA-31/fluent_typed_codegen/tree/feat/runtime-module-loading/examples/minimal)
/// provides a complete consumer. Attributes, Rust visibility and a
/// trailing semicolon inside the invocation are optional. The macro only declares
/// the module, imports the Fluent libraries and includes Cargo output; it neither
/// generates sources nor loads catalogs. Explicit low-level
/// inclusion remains supported for custom dependency aliases or frontend layouts.
/// For selective embedding, call
/// `texts::embed_manifest!(module = texts::presentation::Hud)` with a generated
/// leaf or group type, or a `use` alias. A declaration alone embeds no FTL,
/// including in unoptimized builds; the no-argument form embeds the whole tree.
#[macro_export]
macro_rules! translations {
	($(#[$attribute:meta])* $visibility:vis mod $name:ident $(;)?) => {
		/// Typed translations generated from the consuming package's catalogs.
		$(#[$attribute])*
		// Upstream emits helpers and argument lists consumers may not use.
		#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
		$visibility mod $name {
			use $crate as __fluent_codegen;
			#[allow(clippy::single_component_path_imports)]
			use ::fluent_syntax;
			#[allow(clippy::single_component_path_imports)]
			use ::fluent_typed;

			::std::include!(::std::concat!(::std::env!("OUT_DIR"), "/translations.rs"));
		}
	};
}

/// Define a crate-local embedded-manifest macro from deferred include expressions.
#[doc(hidden)]
#[macro_export]
// Rustfmt repeatedly indents the nested dollar-token matcher on every pass.
#[rustfmt::skip]
macro_rules! __define_embed_manifest {
	($recipe:expr; $dollar:tt) => {
		#[allow(unused_macros)]
		macro_rules! __embed_manifest {
			() => {
				$crate::LocalizationManifest::__embedded($recipe)
			};
			(module = $dollar scope:path $dollar(,)?) => {{
				// Resolve the type as well as its generated companion macro.
				let _: ::std::option::Option<$dollar scope> = ::std::option::Option::None;
				$dollar scope!(@manifest)
			}};
		}

		/// Explicitly include this package's build-time FTL in the calling crate.
		/// With no arguments, includes all modules in all discovered languages.
		/// `module = path::ToScope` selects a generated leaf, group or root type;
		/// `use` aliases work too. Only the selected scope's includes expand,
		/// independently of compiler optimization or linker dead-code removal.
		/// Available only within the crate declaring `translations!`.
		#[allow(unused_imports)]
		pub(crate) use __embed_manifest as embed_manifest;
	};
}

/// Associate a deferred recipe with a generated type in Rust's macro namespace.
#[doc(hidden)]
#[macro_export]
macro_rules! __define_embed_scope {
	($scope:ident, $internal:ident, $recipe:expr) => {
		#[allow(unused_macros)]
		macro_rules! $internal {
			(@manifest) => {
				$crate::LocalizationManifest::__embedded($recipe)
			};
		}

		#[doc(hidden)]
		#[allow(unused_imports)]
		pub(crate) use $internal as $scope;
	};
}
