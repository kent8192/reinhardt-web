//! Macro call AST nodes for special macros in page! DSL.
//!
//! This module defines AST nodes for special macro calls like `static!("path")`.
//! These macros provide integration with other reinhardt crates (static files, URLs, etc.).

use proc_macro2::Span;
use syn::{Expr, Ident};

/// Untyped macro call node.
///
/// Represents a special macro invocation in the page! DSL, such as:
/// - `static!("images/logo.png")`
/// - `url!("home")`
///
/// # Example
///
/// ```text
/// img {
///     src: static!("logo.png"),
///     alt: "Logo"
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PageMacroCall {
	/// The macro name (e.g., "static", "url")
	pub name: Ident,
	/// Arguments passed to the macro
	pub args: Vec<Expr>,
	/// Source span for error reporting
	pub span: Span,
}

impl PageMacroCall {
	/// Creates a new macro call node.
	pub fn new(name: Ident, args: Vec<Expr>, span: Span) -> Self {
		Self { name, args, span }
	}
}

/// Typed macro call node (after validation).
///
/// This is the validated version of [`PageMacroCall`] that has passed
/// through semantic validation.
#[derive(Debug, Clone)]
pub struct TypedMacroCall {
	/// The macro name (e.g., "static", "url")
	pub name: Ident,
	/// Validated arguments
	pub args: Vec<Expr>,
	/// Source span for error reporting
	pub span: Span,
}

impl TypedMacroCall {
	/// Creates a new typed macro call node.
	pub fn new(name: Ident, args: Vec<Expr>, span: Span) -> Self {
		Self { name, args, span }
	}
}
