//! Semantic validation and type transformation.
//!
//! This module transforms untyped AST into typed AST while performing
//! semantic validation and type checking.

pub mod error;
mod form;
mod head;
mod html_spec;
mod page;

pub use error::*;
pub use form::{
	validate_form, validate_form_with_ambient_arguments_source, validate_redirect_on_success,
};
pub use head::validate_head;
pub use page::validate_page;

// Re-export typed AST types from core
pub use crate::core::TypedHeadMacro;
