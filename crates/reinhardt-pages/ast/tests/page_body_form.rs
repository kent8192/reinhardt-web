use proc_macro2::{Punct, Spacing};
use quote::quote;
use reinhardt_pages_ast::{PageMacro, PageMacroForm};

#[test]
fn page_body_form_parses_bare_block_as_implicit_body() {
	let parsed: PageMacro = syn::parse2(quote!({
		div { "Hello" }
	}))
	.unwrap();

	assert!(parsed.is_implicit_body());
	assert!(matches!(parsed.form, PageMacroForm::ImplicitBody { .. }));
}

#[test]
fn page_body_form_parses_head_with_bare_block_as_implicit_body() {
	let head_marker = Punct::new('#', Spacing::Alone);
	let parsed = syn::parse2::<PageMacro>(quote!(
		#head_marker head: page_head,
		{ main { "Project settings" } }
	))
	.unwrap();

	assert!(parsed.head.is_some());
	assert!(parsed.is_implicit_body());
	assert!(matches!(parsed.form, PageMacroForm::ImplicitBody { .. }));
}

#[test]
fn page_body_form_keeps_strict_closure_as_callable_factory() {
	let parsed: PageMacro = syn::parse2(quote!(|| {
		div { "Static" }
	}))
	.unwrap();

	assert!(!parsed.is_implicit_body());
	assert!(matches!(parsed.form, PageMacroForm::StrictClosure { .. }));
}
