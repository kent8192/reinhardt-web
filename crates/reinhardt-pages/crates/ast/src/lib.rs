//! AST definitions for the page! macro DSL.
//!
//! This crate provides the Abstract Syntax Tree (AST) structures and parsing logic
//! for the `page!` macro's Domain Specific Language (DSL). It is designed to be
//! shared between the proc-macro crate and other tools like formatters.
//!
//! ## DSL Structure
//!
//! ```text
//! // Basic structure
//! page!(|props| {
//!     element {
//!         attr: value,
//!         @event: handler,
//!         child_element { ... }
//!         "text content"
//!         { expression }
//!     }
//! })
//!
//! // With head directive (for SSR)
//! page! {
//!     #head: head_expr,
//!     |props| {
//!         element { ... }
//!     }
//! }
//! ```
//!
//! ## Main Types
//!
//! ### Untyped AST (from parser)
//!
//! - [`PageMacro`] - The top-level AST node representing the entire macro invocation
//! - [`PageNode`] - A node in the view tree (element, text, expression, control flow, component)
//! - [`PageElement`] - An HTML element with attributes, events, and children
//! - [`PageAttr`] - An attribute on an element
//! - [`PageEvent`] - An event handler on an element
//! - [`PageIf`] - Conditional rendering
//! - [`PageFor`] - List rendering
//! - [`PageWatch`] - Reactive watch block for Signal-dependent expressions
//! - [`PageComponent`] - A component call
//!
//! ### Typed AST (from validator)
//!
//! - [`TypedPageMacro`] - Validated and typed AST with type-safe attributes
//! - [`TypedPageNode`] - Typed nodes with validated attribute values
//! - [`TypedPageElement`] - Element with typed attributes
//! - [`TypedPageAttr`] - Attribute with typed value
//! - [`TypedPageWatch`] - Reactive watch block for Signal-dependent expressions
//! - [`AttrValue`] - Typed representation of attribute values
//!
//! ## Usage
//!
//! ```rust,ignore
//! use reinhardt_pages_ast::PageMacro;
//! use syn::parse2;
//! use quote::quote;
//!
//! let tokens = quote! {
//!     |name: String| {
//!         div {
//!             class: "container",
//!             h1 { "Hello, " name }
//!         }
//!     }
//! };
//!
//! let page_macro: PageMacro = parse2(tokens).unwrap();
//! ```

mod node;
mod parser;
pub mod typed_node;
pub mod types;

pub use node::{
	PageAttr, PageBody, PageComponent, PageComponentArg, PageElement, PageElse, PageEvent,
	PageExpression, PageFor, PageIf, PageMacro, PageNode, PageParam, PageText, PageWatch,
	debug_tokens,
};
pub use typed_node::{
	TypedPageAttr, TypedPageBody, TypedPageComponent, TypedPageElement, TypedPageElse,
	TypedPageFor, TypedPageIf, TypedPageMacro, TypedPageNode, TypedPageWatch,
};
