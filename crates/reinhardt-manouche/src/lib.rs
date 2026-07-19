#![warn(missing_docs)]
//! Manouche DSL compiler front-end for the `reinhardt-pages` macro family.
//!
//! This crate provides the Abstract Syntax Tree (AST) structures,
//! parsing logic, and semantic validation for the `page!`, `form!`,
//! `head!`, and `style!` macros. Final Rust code generation is performed by
//! downstream consumers (e.g. `reinhardt-pages/macros`) directly from
//! the Typed AST.
//!
//! The name "manouche" comes from [Manouche Jazz](https://en.wikipedia.org/wiki/Gypsy_jazz),
//! a genre of music created by Django Reinhardt in the 1930s.
//!
//! ## Modules
//!
//! - [`core`] - DSL types, Untyped/Typed AST, reactive traits
//! - [`parser`] - TokenStream -> Untyped AST
//! - [`style`] - checked style diagnostics, normative registries,
//!   deterministic scoping, structured CSS lowering, and serialization
//! - [`validator`] - Untyped AST -> Typed AST (semantic analysis)
//!
//! ## Pipeline
//!
//! ```text
//! page!/form!/head!: TokenStream -> parse -> Untyped AST -> validate -> Typed AST
//! style!:            TokenStream -> parse -> validate -> scope -> CSS IR -> CSS
//! ```
//!
//! The style property, unit, function, and value-grammar registries live in
//! this crate. Proc macros and static extraction both call [`compile_style`],
//! so stable [`StyleDiagnosticKind`] values, scoped names, and generated CSS
//! cannot drift between consumers. Source formatting uses a separate,
//! non-semantic parser and never replaces compilation or validation.
//! Callers should match structured diagnostic variants rather than diagnostic
//! display text: syntax failures use [`StyleDiagnosticKind::Syntax`], while
//! validation and lowering retain their specific variants and source spans.
//!
//! ## Component style compiler
//!
//! Supply the consuming package identity and authored style type through
//! [`StyleCompileContext`], then serialize the opaque checked stylesheet. Its
//! three fields form the deterministic scope identity:
//!
//! ```
//! use quote::quote;
//! use reinhardt_manouche::{StyleCompileContext, compile_style, serialize_css};
//!
//! let compiled = compile_style(
//!     quote! {
//!         vars { accent: Color = red; }
//!         .card { color: vars.accent; }
//!     },
//!     &StyleCompileContext {
//!         package_name: "poll-app",
//!         package_version: "0.4.0",
//!         style_type_name: "PollCardStyles",
//!     },
//! )
//! .expect("style definition should compile");
//!
//! assert_eq!(compiled.scope.suffix, "f69b9cbc74c9");
//! assert_eq!(compiled.classes[0].css_name, "card--rs-f69b9cbc74c9");
//! assert_eq!(
//!     serialize_css(&compiled.css),
//!     concat!(
//!         ".card--rs-f69b9cbc74c9 {\n",
//!         "  color: var(--rs-f69b9cbc74c9-accent, red);\n",
//!         "}\n",
//!     )
//! );
//! ```

pub mod core;
pub mod hot_reload;
pub mod parser;
pub mod style;
pub mod validator;

// Convenience re-exports from core
pub use core::*;
// Convenience re-exports from the checked style registry.
pub use style::{
	ArgumentConstraints, ArityPolicy, CompiledStyle, CssStylesheet, FunctionResult, FunctionSpec,
	GrammarMember, LoweringStrategy, PropertyFamily, PropertySpec, ReservedFunction, ScopedClass,
	ScopedVariable, StyleCompileContext, StyleDiagnostic, StyleDiagnosticKind, StyleRelatedLabel,
	StyleScope, UnitCategory, UnitSpec, ValueGrammar, compile_style, function_specs,
	property_specs, registry_reference_text, serialize_css, unit_specs,
};
