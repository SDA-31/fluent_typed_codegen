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
/// let translations: texts::Translations = texts::Locale::En.load();
/// ```
///
/// This example needs application-owned FTL and build output; the runnable
/// `examples/minimal` package compiles it. Attributes, Rust visibility and a
/// trailing semicolon inside the invocation are optional. The macro only declares
/// the module, imports the Fluent libraries and includes Cargo output; it neither
/// generates sources nor installs a framework/provider. Explicit low-level
/// inclusion remains supported for custom dependency aliases or frontend layouts.
#[macro_export]
macro_rules! translations {
	($(#[$attribute:meta])* $visibility:vis mod $name:ident $(;)?) => {
		/// Typed translations generated from the consuming package's catalogs.
		$(#[$attribute])*
		// Upstream emits helpers and argument lists consumers may not use.
		#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
		$visibility mod $name {
			#[allow(clippy::single_component_path_imports)]
			use ::fluent_syntax;
			#[allow(clippy::single_component_path_imports)]
			use ::fluent_typed;

			::std::include!(::std::concat!(::std::env!("OUT_DIR"), "/translations.rs"));
		}
	};
}
