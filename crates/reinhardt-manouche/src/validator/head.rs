//! Validation for head! macro.

use syn::Result;

use crate::core::HeadMacro;

/// Typed head macro AST.
///
/// For now, this is the same as the untyped AST since head validation
/// is relatively simple (mostly element name validation).
pub type TypedHeadMacro = HeadMacro;

/// Validates a `HeadMacro`.
///
/// Currently performs validation without transformation since head elements
/// don't require complex type transformations like page or form elements.
pub fn validate_head(_ast: &HeadMacro) -> Result<TypedHeadMacro> {
	// TODO: Implement head validation
	todo!("Implement head validator")
}
