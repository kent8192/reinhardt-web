//! AST definitions for the page! macro DSL.
//!
//! This crate provides the Abstract Syntax Tree (AST) structures and parsing logic
//! for the `page!` macro's Domain Specific Language (DSL). It is designed to be
//! shared between the proc-macro crate and other tools like formatters.
//!
//! ## DSL Structure
//!
//! ```text
//! page!(|props| {
//!     element {
//!         attr: value,
//!         @event: handler,
//!         child_element { ... }
//!         "text content"
//!         { expression }
//!     }
//! })
//! ```
//!
//! ## Main Types
//!
//! - [`PageMacro`] - The top-level AST node representing the entire macro invocation
//! - [`PageNode`] - A node in the view tree (element, text, expression, control flow, component)
//! - [`PageElement`] - An HTML element with attributes, events, and children
//! - [`PageAttr`] - An attribute on an element
//! - [`PageEvent`] - An event handler on an element
//! - [`PageIf`] - Conditional rendering
//! - [`PageFor`] - List rendering
//! - [`PageComponent`] - A component call
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

pub use node::{
	PageAttr, PageBody, PageComponent, PageComponentArg, PageElement, PageElse, PageEvent,
	PageExpression, PageFor, PageIf, PageMacro, PageNode, PageParam, PageText, debug_tokens,
};
