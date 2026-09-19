//! Validated single-resource updates using caller-owned scope and execution.

use std::marker::PhantomData;

use reinhardt_core::exception::Error;
use reinhardt_core::model_form::PatchValidationError;
use reinhardt_db::orm::{
	FieldAssignment, Filter, FilterOperator, IntoPrimaryKey, Model, OrmExecutor, QuerySet,
};

/// A failure before or during a validated patch write.
///
/// **Parity: P0.** Native-only; database errors retain their structured metadata.
#[derive(Debug, thiserror::Error)]
pub enum PatchError {
	/// Field or application validation failed.
	#[error(transparent)]
	Validation(#[from] PatchValidationError),
	/// No assignments were submitted; no UPDATE is executed.
	#[error("A patch requires at least one submitted field")]
	EmptyPatch,
	/// The existing validation snapshot has no primary key.
	#[error("The validation snapshot requires a primary key")]
	MissingSnapshotKey,
	/// The target differs from the resource whose snapshot was validated.
	#[error("The patch target differs from the validation snapshot")]
	TargetMismatch,
	/// Composite-key persistence is outside this single-key bridge.
	#[error("Composite-key models are not supported by form patches")]
	CompositePrimaryKey,
	/// The conditional query or backend operation failed.
	#[error(transparent)]
	Database(#[from] Error),
}

/// The backend-reported write outcome, not a transaction commit receipt.
///
/// **Parity: P0.** Zero rows does not identify a missing, forbidden, or stale row.
/// Same-value writes follow backend/driver count semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatchOutcome {
	/// Backend-reported affected rows; not a portable changed-value count.
	pub rows_affected: u64,
}

/// Native code-generation bridge for an allowlisted form contract.
///
/// **Parity: P0.** Implementations are trusted application code, not wire input.
/// The generated implementation validates raw data before producing assignments.
/// No public API accepts a client-provided assignment list as a validated patch.
#[doc(hidden)]
pub trait ModelFormPatchContract<M: Model> {
	/// The strict raw payload generated for this contract.
	type Data;
	/// Checks submission presence before validation or ORM short circuits.
	fn is_empty(data: &Self::Data) -> bool;
	/// Validates raw input and maps submitted values through typed model fields.
	fn validated_assignments(
		data: Self::Data,
		existing: Option<&M>,
	) -> Result<Vec<FieldAssignment>, PatchError>;
}

/// An opaque, server-validated set of assignments for one model and form.
///
/// **Parity: P0.** Native-only; this type is not deserializable. Construction
/// always runs the contract's server validation. Applying consumes the patch.
///
/// Private fields prevent unchecked construction:
///
/// ```compile_fail
/// use reinhardt_forms::ValidatedFormPatch;
/// use reinhardt_db::orm::Model;
/// fn forge<M: Model, C>() -> ValidatedFormPatch<M, C> {
///     ValidatedFormPatch {
///         assignments: Vec::new(),
///         snapshot_key: None,
///         contract: std::marker::PhantomData,
///     }
/// }
/// ```
///
/// Client data cannot deserialize into this capability:
///
/// ```compile_fail
/// use reinhardt_forms::ValidatedFormPatch;
/// use reinhardt_db::orm::Model;
/// fn decode<M: Model, C>(json: &str) -> ValidatedFormPatch<M, C> {
///     serde_json::from_str(json).unwrap()
/// }
/// ```
///
/// The model type must match the supplied QuerySet:
///
/// ```compile_fail
/// use reinhardt_forms::{ValidatedFormPatch, PatchError};
/// use reinhardt_db::orm::{Model, OrmExecutor, QuerySet};
/// async fn wrong_model<A: Model, B: Model, C, E: OrmExecutor>(
///     patch: ValidatedFormPatch<A, C>, scope: QuerySet<B>, target: &A, executor: &mut E,
/// ) where A::PrimaryKey: PartialEq {
///     patch.apply_to(scope, target, executor).await.unwrap();
/// }
/// ```
pub struct ValidatedFormPatch<M: Model, C> {
	assignments: Vec<FieldAssignment>,
	snapshot_key: Option<M::PrimaryKey>,
	contract: PhantomData<fn() -> C>,
}

impl<M: Model, C: ModelFormPatchContract<M>> ValidatedFormPatch<M, C> {
	/// Checked factory used by generated form markers.
	///
	/// **Parity: P0.** Raw input is validated even when a client validated it first.
	#[doc(hidden)]
	pub fn validate(data: C::Data, existing: Option<&M>) -> Result<Self, PatchError> {
		if C::is_empty(&data) {
			return Err(PatchError::EmptyPatch);
		}
		if M::composite_primary_key().is_some() {
			return Err(PatchError::CompositePrimaryKey);
		}
		let snapshot_key = existing
			.map(|model| model.primary_key().ok_or(PatchError::MissingSnapshotKey))
			.transpose()?;
		let assignments = C::validated_assignments(data, existing)?;
		if assignments.is_empty() {
			return Err(PatchError::EmptyPatch);
		}
		Ok(Self {
			assignments,
			snapshot_key,
			contract: PhantomData,
		})
	}
}

impl<M: Model, C> ValidatedFormPatch<M, C> {
	/// Applies this patch to a single key within the caller's complete scope.
	///
	/// **Parity: P0.** Uses the supplied executor without reads, implicit commit,
	/// locks, version increments, or full-model saves. Existing predicates remain
	/// attached to the write, ANDed with the explicit target's primary key.
	///
	/// # Panics
	///
	/// Inherits `IntoPrimaryKey` preconditions: `None` and references to models
	/// without primary keys panic in their existing conversion implementations.
	pub async fn apply_to<K, E>(
		self,
		scoped: QuerySet<M>,
		target: K,
		executor: &mut E,
	) -> Result<PatchOutcome, PatchError>
	where
		K: IntoPrimaryKey<M>,
		E: OrmExecutor,
		M::PrimaryKey: PartialEq,
	{
		let key = target.into_primary_key();
		if self
			.snapshot_key
			.as_ref()
			.is_some_and(|snapshot| snapshot != &key)
		{
			return Err(PatchError::TargetMismatch);
		}
		let query = scoped.filter(Filter::new(
			M::primary_key_column(),
			FilterOperator::Eq,
			M::primary_key_filter_value(key),
		));
		let rows_affected = query
			.update_fields_with_conn(executor, self.assignments)
			.await?;
		Ok(PatchOutcome { rows_affected })
	}
}
