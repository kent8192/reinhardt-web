//! Typed AST to IR lowering.

use crate::core::{TypedFormMacro, TypedPageMacro};
use crate::validator::TypedHeadMacro;

use super::{ComponentIR, FormIR, HeadIR};

/// Lowers a typed page AST to IR.
pub fn lower_page(_typed: &TypedPageMacro) -> ComponentIR {
	// TODO: Implement lowering
	todo!("Implement page lowering")
}

/// Lowers a typed form AST to IR.
pub fn lower_form(_typed: &TypedFormMacro) -> FormIR {
	// TODO: Implement lowering
	todo!("Implement form lowering")
}

/// Lowers a typed head AST to IR.
pub fn lower_head(_typed: &TypedHeadMacro) -> HeadIR {
	// TODO: Implement lowering
	todo!("Implement head lowering")
}
