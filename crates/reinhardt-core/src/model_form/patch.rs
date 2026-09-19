//! Update-only validation contracts, without persistence capabilities.

use crate::validators::ValidationErrors;

/// A failure while validating submitted update values.
///
/// **Parity: P2.** These errors preserve the same field details on native and WASM.
#[derive(Debug, thiserror::Error)]
pub enum PatchValidationError {
	/// A generated field or application validator rejected the candidate.
	#[error(transparent)]
	Validation(#[from] ValidationErrors),
	/// A model-wide validator requires an explicit existing-value snapshot.
	#[error("Existing values are required to validate this patch")]
	ExistingValuesRequired,
	/// A create-cleaned payload still contains materialized defaults.
	#[error("Create defaults cannot be submitted as patch values")]
	DefaultedValues,
}

/// Validates only submitted fields, without applying create defaults.
///
/// **Parity: P2.** The context contains shared form values, not an ORM model.
/// Validation is advisory on the client; native persistence always validates
/// raw input again. The cleaned result is not a persistence capability.
pub trait ModelFormPatchPayload: Sized {
	/// Normalized partial values; omitted fields remain omitted.
	type Cleaned;
	/// Explicit existing values used by model-wide validators.
	type Context;

	/// Normalizes submitted fields and validates the resulting candidate.
	///
	/// A supplied context contributes validation values only, never assignments.
	fn clean_and_validate_patch(
		self,
		existing: Option<&Self::Context>,
	) -> Result<Self::Cleaned, PatchValidationError>;
}
