use super::format;
use quote::quote;

#[test]
fn opaque_syntax_is_a_diagnostic_including_inside_nested_scopes() {
	for tokens in [
		quote!(
			fn declared();
		),
		quote!(
			mod nested {
				fn declared();
			}
		),
		quote!(
			fn outer() {
				fn declared();
			}
		),
		quote!(
			use {::std::fmt};
		),
	] {
		let error = format(tokens, "adapter.rs").unwrap_err().to_string();

		assert!(error.contains("generated Rust `adapter.rs`"), "{error}");
		assert!(error.contains("Verbatim"), "{error}");
	}
}

#[test]
fn macro_tokens_are_not_mistaken_for_unsupported_rust_syntax() {
	let output = format(
		quote! {
			macro_rules! custom {
				($value:expr) => { $value };
			}

			fn value() -> u8 {
				custom!(7)
			}
		},
		"adapter.rs",
	)
	.unwrap();

	assert!(output.contains("fn value() -> u8"));
	assert!(syn::parse_file(&output).is_ok());
}
