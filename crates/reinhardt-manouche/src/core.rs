//! Core DSL types, AST definitions, and reactive traits.
//!
//! This module contains:
//! - Untyped AST nodes (`PageMacro`, `FormMacro`, `HeadMacro`, `StyleMacro`)
//! - Typed AST nodes (`TypedPageMacro`, `TypedFormMacro`, etc.)
//! - Reactive primitive traits (`Signal`, `Effect`, `Memo`)
//! - Common types and utilities

pub mod attr_utils;
pub mod form_node;
pub mod form_typed;
pub mod head_node;
pub mod node;
pub mod reactive;
pub mod style_node;
pub mod style_typed;
pub mod typed_node;
pub mod types;

pub use form_node::*;
pub use form_typed::*;
pub use head_node::*;
pub use node::*;
pub use reactive::*;
pub use style_node::*;
pub use style_typed::{
	Direction, KeywordDomain, NumericConstraint, NumericDimension, SemanticType, StyleRuntimeType,
	TypeConstraint, TypedFunctionCall, TypedGlobalReference, TypedStyleClass,
	TypedStyleDeclaration, TypedStyleGlobal, TypedStyleItem, TypedStyleMacro, TypedStyleMediaRule,
	TypedStyleRule, TypedStyleRuleItem, TypedStyleVariable, TypedValueExpr, TypedValueExprKind,
	TypedVariableReference,
};
pub use typed_node::*;
pub use types::*;
