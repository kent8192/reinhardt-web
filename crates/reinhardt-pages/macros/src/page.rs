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
//!
//! ## Component invocation (spec §3.5)
//!
//! Two syntactically distinct forms can be used to invoke a component from
//! within a `page!` body:
//!
//! 1. **Legacy positional / paren form** — `{my_button("label".into(), false)}`.
//!    The component is just a normal Rust function call wrapped in `{ ... }`.
//!
//! 2. **React-style brace form** — `Card { item: x, @click: h, p { "kid" } }`.
//!    The component is a function `fn card(props: CardProps) -> Page` where
//!    `CardProps` derives `bon::Builder`. Codegen emits a builder chain that
//!    sets each named prop, each `@event:` prop (as `.on_<event>(handler)`),
//!    and (when children are present) `.children(Some(<child_view>))`.
//!
//! Both forms coexist; the parser picks based on the punctuation that
//! follows the component identifier (`(` vs. `{`).
//!
//! `ignore` — this doctest lives in a proc-macro crate where
//! `reinhardt_pages::*` imports aren't resolvable and `bon` isn't a
//! dev-dependency. The example is tested via integration tests instead.
//!
//! ```ignore
//! use reinhardt_pages::component::Page;
//! use reinhardt_pages::page;
//!
//! #[derive(bon::Builder)]
//! struct CardProps { item: String }
//!
//! fn card(p: CardProps) -> Page {
//!     page!(|p: CardProps| { article { h2 { {p.item.clone()} } } })(p)
//! }
//!
//! // Brace form (spec §3.5).
//! let _ = page!(|| { div { Card { item: "hello".to_string() } } });
//! ```
//!
//! See `reinhardt-pages/CHANGELOG.md` `### Added` entry and the design
//! comment in `page::codegen::generate_component_brace` for the full
//! lowering rules.

mod codegen;
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

	// 2. Validate + Transform: Untyped AST → Typed AST
	let typed_ast = match validator::validate(&untyped_ast) {
		Ok(ast) => ast,
		Err(err) => return err.to_compile_error().into(),
	};

	// 3. Codegen: Typed AST → Rust code
	let output = codegen::generate(&typed_ast);

	output.into()
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
