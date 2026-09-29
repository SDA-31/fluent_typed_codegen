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
/// [runnable example](https://github.com/SDA-31/fluent_typed_codegen/tree/main/examples/minimal)
/// provides a complete consumer. Attributes, Rust visibility and a
/// trailing semicolon inside the invocation are optional. The macro only declares
/// the module, imports the Fluent libraries and includes Cargo output; it neither
/// generates sources nor loads catalogs. Explicit low-level
/// inclusion remains supported for custom dependency aliases or frontend layouts.
/// Named manifests can be declared with a selector relative to this tree:
/// `texts::embed_manifest! { pub const HUD = presentation::Hud; }`.
/// The constant has type [`crate::LocalizationManifest`] and can be exported
/// independently of the generated module. Ordinary catalog aliases remain usable.
/// Selectors are relative schema paths, not application imports or type aliases.
/// A `translations!` declaration alone embeds no FTL, including in unoptimized
/// builds; `embed_manifest!()` embeds the whole tree.
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
	($recipe:expr; $selectors:tt; $recipes:tt; $dollar:tt) => {
		/// Explicitly include build-time FTL in the calling crate.
		///
		/// Declare named manifests with relative selectors:
		/// `embed_manifest! { pub const HUD = presentation::Hud; }`.
		/// Each constant is a `LocalizationManifest`; visibility, attributes and
		/// multiple declarations are supported. Selectors belong to this generated
		/// tree, independently of application imports. `Translations` selects its root.
		/// A declaration embeds source bytes, without parsing a Fluent catalog.
		///
		/// `embed_manifest!()` includes all modules in all discovered languages.
		/// For a subset, declare constants using relative leaf or group selectors.
		/// Ordinary type aliases remain usable for catalog construction and resources.
		/// Only the selected scope's includes expand, independently of optimization.
		/// Available only within the crate declaring `translations!`.
		#[allow(unused_macros)]
		macro_rules! __embed_manifest {
			($dollar(
				$dollar(#[$dollar attribute:meta])*
				$dollar visibility:vis const $dollar name:ident = $dollar($dollar scope:ident)::+;
			)+) => {
				$dollar(
					$crate::__declare_embedded!(
						$selectors; $recipes;
						$dollar(#[$dollar attribute])*
						$dollar visibility const $dollar name = $dollar($dollar scope)::+;
					);
				)+
			};
			($dollar(#[$dollar attribute:meta])* $dollar visibility:vis const $dollar($dollar invalid:tt)*) => {
				::core::compile_error!("embed_manifest!: expected `pub const NAME = relative::Scope;`. Use a catalog selector relative to this translation tree, without strings, generic arguments or a leading `::`.");
			};
			() => {
				$crate::LocalizationManifest::__embedded($recipe)
			};
			($dollar($dollar invalid:tt)+) => {
				::core::compile_error!("embed_manifest!: use `embed_manifest!()` for the complete tree or `embed_manifest! { pub const NAME = relative::Scope; }` for selected scopes.");
			};
		}

		#[allow(unused_imports)]
		pub(crate) use __embed_manifest as embed_manifest;
	};
}

/// Declare one const manifest, keeping unselected include expressions in macro arms.
#[doc(hidden)]
#[macro_export]
macro_rules! __declare_embedded {
	(
		$selectors:tt;
		{ $(($($known:ident)::+) => $recipe:expr,)* };
		$(#[$attribute:meta])* $visibility:vis const $name:ident = $($scope:ident)::+;
	) => {
		$(#[$attribute])*
		$visibility const $name: $crate::LocalizationManifest = {
			// Qualifying through this local namespace keeps completion and resolution
			// inside the generated schema even when the application shadows its names.
			#[allow(dead_code)]
			mod __fluent_selectors $selectors

			let _: ::core::marker::PhantomData<__fluent_selectors::$($scope)::+> = ::core::marker::PhantomData;

			macro_rules! __fluent_recipe {
				$(($($known)::+) => { $recipe };)*
				($($scope)::+) => {
					::core::compile_error!(::core::concat!(
						"embed_manifest!: unknown relative catalog selector `", ::core::stringify!($($scope)::+),
						"`. Use a selector from this translation tree, for example presentation::Hud, or Translations for the complete tree."
					))
				};
			}

			// Discovery orders unique entries by locale and then logical module path;
			// filtering a scope preserves that order for allocation-free binary lookup.
			const RECIPE: (
				&str, &str, &str,
				&[(&str, &str, &[u8])],
			) = __fluent_recipe!($($scope)::+);
			static CONFIG: ::std::sync::LazyLock<$crate::CatalogConfig> = ::std::sync::LazyLock::new(|| {
				$crate::CatalogConfig {
					source_language: RECIPE.0.into(),
					default_language: RECIPE.1.into(),
					languages_directory: RECIPE.2.into(),
				}
			});

			$crate::LocalizationManifest::__embedded_static(RECIPE.3, &CONFIG)
		};
	};
}
