//! Typed AST to IR lowering.

use crate::core::{TypedFormMacro, TypedPageMacro};
use crate::validator::TypedHeadMacro;

use super::{ComponentIR, FormIR, HeadIR};

/// Error type for IR lowering operations.
#[derive(Debug)]
pub enum LowerError {
	/// Indicates that page lowering has not yet been implemented.
	UnimplementedPage,
	/// Indicates that form lowering has not yet been implemented.
	UnimplementedForm,
	/// Indicates that head lowering has not yet been implemented.
	UnimplementedHead,
}

impl std::fmt::Display for LowerError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::UnimplementedPage => write!(f, "Page lowering is not yet implemented"),
			Self::UnimplementedForm => write!(f, "Form lowering is not yet implemented"),
			Self::UnimplementedHead => write!(f, "Head lowering is not yet implemented"),
		}
	}
}

impl std::error::Error for LowerError {}

/// Convenient result alias for IR lowering.
pub type LowerResult<T> = Result<T, LowerError>;

/// Lowers a typed page AST to IR.
pub fn lower_page(_typed: &TypedPageMacro) -> LowerResult<ComponentIR> {
	// TODO: Implement page lowering.
	// For now, return a structured error instead of panicking.
	Err(LowerError::UnimplementedPage)
}

/// Lowers a typed form AST to IR.
pub fn lower_form(_typed: &TypedFormMacro) -> LowerResult<FormIR> {
	// TODO: Implement form lowering.
	// For now, return a structured error instead of panicking.
	Err(LowerError::UnimplementedForm)
}

/// Lowers a typed head AST to IR.
pub fn lower_head(_typed: &TypedHeadMacro) -> LowerResult<HeadIR> {
	// TODO: Implement head lowering.
	// For now, return a structured error instead of panicking.
	Err(LowerError::UnimplementedHead)
}
