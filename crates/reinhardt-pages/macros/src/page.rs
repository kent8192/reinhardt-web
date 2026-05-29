//! The `page!` macro implementation.
//!
//! This module provides the `page!` procedural macro for creating anonymous
//! WASM components with a concise, ergonomic DSL.
//!
//! ## v2 contract
//!
//! Per the Manouche v2 design, `page!` enforces three rules at compile time:
//!
//! 1. **No implicit captures** (spec §3.7). Every value identifier inside
//!    the body must appear in the closure parameter list. Item paths
//!    (multi-segment like `crate::util::fmt`), type identifiers (`Vec`,
//!    `Option`), constants (`MAX_LEN`), and macro invocations (`format!`)
//!    are exempt. Free function calls should use `self::` (or any module
//!    prefix) so the path is multi-segment.
//!
//! 2. **Unconditional auto-wrap** (spec §4.1). Every `{expr}` and every
//!    `if` / `for` control-flow block is wrapped in
//!    `Page::reactive(move || ...)` at codegen time. Re-renders happen
//!    automatically when tracked inputs change. The historical `watch { ... }`
//!    wrapper is removed.
//!
//! 3. **No bare-identifier shorthand** (spec §3.6). Bare identifiers in
//!    element bodies are no longer accepted. The shorthand `div { foo }`
//!    was ambiguous with the element form `div { foo { ... } }`, so the
//!    validator requires the explicit braced form:
//!
//!    - Before (v1): `` `div { name }` ``
//!    - After (v2):  `` `div { {name} }` ``
//!
//!    The codemod `cargo make migrate-manouche-v2` (PR3) rewrites existing
//!    sources mechanically.
//!
//! `for` blocks may attach a stable reconciliation key with `@key(expr)`.
//! The key expression is evaluated for each item and stored in a
//! `Page::KeyedFragment`; unkeyed `for` blocks keep producing regular
//! fragments.
//!
//! Because every `for` block is auto-wrapped in `Page::reactive(move || ...)`
//! (rule 2 above), the iterator expression is cloned on each reactive re-run.
//! The iterator expression must therefore implement `Clone` (e.g. `Vec<T>`,
//! `&[T]`, or any `Clone` collection).
//!
//! ```ignore
//! page!(|todos: Vec<Todo>| {
//!     ul {
//!         // Iterator expression `todos` must implement `Clone`.
//!         for todo in todos @key(todo.id.clone()) {
//!             li { { todo.title.clone() } }
//!         }
//!     }
//! });
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pages::page;
//! use reinhardt_pages::reactive::Signal;
//!
//! // Anonymous component with explicit Signal dependency.
//! let counter = page!(|count: Signal<i32>| {
//!     div {
//!         class: "counter",
//!         h1 { "Counter" }
//!         span { { format!("Count: {}", count.get()) } }
//!         button {
//!             @click: |_| { /* increment logic */ },
//!             "+"
//!         }
//!     }
//! });
//!
//! // Use it like a function.
//! let count = Signal::new(0);
//! let view = counter(count);
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
				span { {name} }
				span { {count.to_string()} }
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
		// Spec §3.7 (no implicit captures): event handler bodies route free
		// functions through `self::` so the path is multi-segment.
		let input = quote!(|| {
			button {
				@click: |e| { self::handle_click(e); },
				@input: |e| { self::handle_input(e); },
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
