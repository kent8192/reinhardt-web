//! The page! macro implementation.
//!
//! This module provides the `page!` procedural macro for creating anonymous
//! WASM components with a concise, ergonomic DSL.
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::page;
//!
//! // Define an anonymous component
//! let counter = page!(|initial: i32| {
//!     div {
//!         class: "counter",
//!         h1 { "Counter" }
//!         span { format!("Count: {}", initial) }
//!         button {
//!             @click: |_| { /* increment logic */ },
//!             "+"
//!         }
//!     }
//! });
//!
//! // Use it like a function
//! let view = counter(42);
//! ```

mod codegen;
pub(crate) mod hook_deps_validator;
pub(crate) mod html_spec;
mod validator;

use proc_macro::TokenStream;

// Re-export PageMacro from the shared ast crate for external use
pub(crate) use reinhardt_manouche::core::PageMacro;

/// Implementation of the page! macro.
///
/// This function processes input through a three-stage pipeline:
/// 1. Parse: TokenStream → Untyped AST
/// 2. Validate + Transform: Untyped AST → Typed AST
/// 3. Codegen: Typed AST → Rust code
pub(crate) fn page_impl(input: TokenStream) -> TokenStream {
	let input2 = proc_macro2::TokenStream::from(input);

	// 1. Parse: TokenStream → Untyped AST
	let untyped_ast: PageMacro = match syn::parse2(input2) {
		Ok(ast) => ast,
		Err(err) => return err.to_compile_error().into(),
	};

	// 1a. Hook deps verification (pre-codegen pass, Refs #4195).
	// Today this emits an empty TokenStream; once the follow-up
	// implementation lands, it will surface `compile_error!` for any
	// Signal read inside a hook closure that is missing from the
	// hook's deps tuple.
	let hook_deps_diagnostics = hook_deps_validator::verify_hook_deps(&untyped_ast);

	// 2. Validate + Transform: Untyped AST → Typed AST
	let typed_ast = match validator::validate(&untyped_ast) {
		Ok(ast) => ast,
		Err(err) => return err.to_compile_error().into(),
	};

	// 3. Codegen: Typed AST → Rust code
	let codegen_output = codegen::generate(&typed_ast);

	// 4. Concatenate verification diagnostics with the codegen output so
	// any `compile_error!` invocations land in the user's source location.
	let combined = quote::quote! {
		#hook_deps_diagnostics
		#codegen_output
	};

	combined.into()
}

#[cfg(test)]
mod tests {
	use super::*;
	use quote::quote;

	#[test]
	fn test_page_macro_basic() {
		let input = quote!(|| { div { "hello" } });
		let untyped_ast: PageMacro = syn::parse2(input).unwrap();
		let typed_ast = validator::validate(&untyped_ast).unwrap();
		let output = codegen::generate(&typed_ast);

		// Verify it generates valid tokens
		assert!(!output.is_empty());
	}

	#[test]
	fn test_page_macro_with_params() {
		let input = quote!(|name: String, count: i32| {
			div {
				class: "greeting",
				span { name }
				span { count.to_string() }
			}
		});
		let untyped_ast: PageMacro = syn::parse2(input).unwrap();
		let typed_ast = validator::validate(&untyped_ast).unwrap();
		let output = codegen::generate(&typed_ast);

		let output_str = output.to_string();
		assert!(output_str.contains("name : String"));
		assert!(output_str.contains("count : i32"));
	}

	#[test]
	fn test_page_macro_with_events() {
		let input = quote!(|| {
			button {
				@click: |e| { handle_click(e); },
				@input: |e| { handle_input(e); },
				"Click me"
			}
		});
		let untyped_ast: PageMacro = syn::parse2(input).unwrap();
		let typed_ast = validator::validate(&untyped_ast).unwrap();
		let output = codegen::generate(&typed_ast);

		let output_str = output.to_string();
		assert!(output_str.contains("EventType"));
		assert!(output_str.contains("Click"));
	}

	#[test]
	fn test_page_macro_nested_elements() {
		let input = quote!(|| {
			div {
				class: "container",
				header {
					h1 { "Title" }
				}
				main {
					p { "Content" }
				}
				footer {
					span { "Footer" }
				}
			}
		});
		let untyped_ast: PageMacro = syn::parse2(input).unwrap();
		let typed_ast = validator::validate(&untyped_ast).unwrap();
		let output = codegen::generate(&typed_ast);

		let output_str = output.to_string();
		assert!(output_str.contains("\"div\""));
		assert!(output_str.contains("\"header\""));
		assert!(output_str.contains("\"main\""));
		assert!(output_str.contains("\"footer\""));
	}
}
