//! AST definitions for the page! and form! macro DSLs.
//!
//! This module re-exports all AST types from the `reinhardt-pages-ast` crate.
//! The AST structures and parsing logic are maintained in a separate crate
//! to be shared between the proc-macro crate and other tools like formatters.
//!
//! ## Main Types
//!
//! ### page! Untyped AST (from parser)
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
//! ### page! Typed AST (from validator)
//!
//! - [`TypedPageMacro`] - Validated and typed AST with type-safe attributes
//! - [`TypedPageNode`] - Typed nodes with validated attribute values
//! - [`TypedPageElement`] - Element with typed attributes
//! - [`TypedPageAttr`] - Attribute with typed value
//! - [`TypedPageWatch`] - Reactive watch block for Signal-dependent expressions
//!
//! ### form! Untyped AST (from parser)
//!
//! - [`FormMacro`] - The top-level form AST node
//! - [`FormFieldDef`] - A field definition with type and properties
//! - [`FormFieldProperty`] - A property within a field (named, flag, widget)
//! - [`FormValidator`] - Server-side validator definition
//! - [`ClientValidator`] - Client-side validator definition
//!
//! ### form! Typed AST (from validator)
//!
//! - [`TypedFormMacro`] - Validated form with typed fields
//! - [`TypedFormFieldDef`] - Validated field with typed properties
//! - [`TypedFieldType`] - Validated field type with Signal mapping
//! - [`TypedWidget`] - Validated widget type
//! - [`TypedFieldStyling`] - Styling properties with defaults
//!
//! ## Usage
//!
//! ```rust,ignore
//! use reinhardt_pages::ast::PageMacro;
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

// Re-export all types from reinhardt-pages-ast
pub use reinhardt_pages_ast::*;
