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
/// For selective embedding, call
/// `texts::embed_manifest!(module = texts::presentation::Hud)` with a generated
/// leaf or group type. The shorter `texts::embed_manifest!(texts::presentation::Hud)`
/// form is also accepted. Imported type names and renamed type paths are not
/// embedded selectors; ordinary catalog imports remain unrestricted. A declaration
/// alone embeds no FTL, including in unoptimized builds; the no-argument form embeds
/// the whole tree.
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
	($recipe:expr; [$($known:ident),*]; $dollar:tt) => {
		/// Explicitly include build-time FTL in the calling crate.
		///
		/// `embed_manifest!()` includes all modules in all discovered languages.
		/// `embed_manifest!(texts::presentation::Hud)` selects a generated type path.
		/// `embed_manifest!(module = texts::presentation::Hud)` is also accepted.
		/// Leaf, group and root paths are supported, including aliases of parent modules.
		/// Use the original generated type name, not an imported or renamed type name.
		/// Ordinary type aliases remain usable for catalog construction and resources.
		/// Only the selected scope's includes expand, independently of optimization.
		/// Available only within the crate declaring `translations!`.
		#[allow(unused_macros)]
		macro_rules! __embed_manifest {
			() => {
				$crate::LocalizationManifest::__embedded($recipe)
			};
			(module = $dollar($dollar scope:tt)*) => {
				$crate::__embed_selected!([$($known),*] $dollar($dollar scope)*)
			};
			($dollar($dollar scope:tt)+) => {
				$crate::__embed_selected!([$($known),*] $dollar($dollar scope)+)
			};
		}

		#[allow(unused_imports)]
		pub(crate) use __embed_manifest as embed_manifest;
	};
}

/// Check the selector as a type and forward its namespace to the generated dispatcher.
#[doc(hidden)]
#[macro_export]
macro_rules! __embed_selected {
	([$($known:ident),*] $scope:ident $(,)?) => {
		::core::compile_error!(
			"embed_manifest!: expected a qualified generated catalog path, for example texts::presentation::Hud. Imported type names and aliases are not supported as embedded selectors; ordinary catalog imports remain valid."
		)
	};
	([$($known:ident),*] $head:ident :: $($tail:ident)::+ $(,)?) => {{
		let _: ::core::marker::PhantomData<$head :: $($tail)::+> = ::core::marker::PhantomData;
		$crate::__embed_scope_path!([$($known),*] [] $head :: $($tail)::+)
	}};
	([$($known:ident),*] :: $head:ident :: $($tail:ident)::+ $(,)?) => {{
		let _: ::core::marker::PhantomData<:: $head :: $($tail)::+> = ::core::marker::PhantomData;
		$crate::__embed_scope_path!([$($known),*] [::] $head :: $($tail)::+)
	}};
	($($unsupported:tt)*) => {
		::core::compile_error!(
			"embed_manifest!: expected a qualified generated catalog path, for example texts::presentation::Hud. Strings, generic arguments and arbitrary expressions are not supported as embedded selectors."
		)
	};
}

/// Walk a qualified selector without interpreting or resolving its type name.
#[doc(hidden)]
#[macro_export]
macro_rules! __embed_scope_path {
	([$($known:ident),*] [$($namespace:tt)*] $head:ident :: $($tail:ident)::+) => {
		$crate::__embed_scope_path!([$($known),*] [$($namespace)* $head ::] $($tail)::+)
	};
	([$($known:ident),*] $namespace:tt $scope:ident) => {{
		// Reject unknown terminal names before resolving a helper in an unrelated module.
		// Keeping the namespace as one token tree avoids nested repetition of its path.
		macro_rules! __fluent_select_known {
			$(
				($known) => { $crate::__embed_scope_path!(@resolve $namespace $scope) };
			)*
			($scope) => {
				::core::compile_error!(::core::concat!(
					"embed_manifest!: unsupported catalog name `", ::core::stringify!($scope),
					"`. Use the original generated type name; renamed types are not supported as embedded selectors."
				))
			};
		}

		__fluent_select_known!($scope)
	}};
	(@resolve [$($namespace:tt)+] $scope:ident) => {
		$($namespace)+ __embed_scope!($scope)
	};
}

/// Define deferred recipes under an implementation name, separate from catalog types.
#[doc(hidden)]
#[macro_export]
#[rustfmt::skip]
macro_rules! __define_embed_scopes {
	($($scope:ident => $recipe:expr,)*; $diagnostic:literal; $dollar:tt) => {
		#[allow(unused_macros)]
		macro_rules! __fluent_embed_scope {
			$(
				($scope) => {
					$crate::LocalizationManifest::__embedded($recipe)
				};
			)*
			($dollar other:ident) => {
				::core::compile_error!(::core::concat!(
					"embed_manifest!: unsupported selector `", ::core::stringify!($dollar other), "`. ", $diagnostic
				))
			};
		}

		#[doc(hidden)]
		#[allow(unused_imports)]
		pub(crate) use __fluent_embed_scope as __embed_scope;
	};
}
