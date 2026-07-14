use super::ServerFnListQuery;

#[cfg(all(native, feature = "model-server-fnset"))]
use super::ServerFnSetError;

#[cfg(all(native, feature = "model-server-fnset"))]
use reinhardt_db::orm::{CustomManager, Model, QuerySet, TransactionExecutor};

/// Cross-target wire contract for one model server function resource.
pub trait ServerFnResource {
	/// Typed detail lookup value.
	///
	/// Detail overrides retain the decoded lookup while the framework performs
	/// the authorized lookup, so the wire value must be reproducible.
	type Lookup: Clone;
	/// Read DTO returned to clients.
	type Read;
	/// Create input DTO.
	type Create;
	/// Full-update input DTO.
	type Update;
	/// Partial-update input DTO.
	type Patch;
	/// Typed list query DTO.
	type ListQuery: ServerFnListQuery;
}

/// Converts a create DTO into a new model value.
#[cfg(all(native, feature = "model-server-fnset"))]
pub trait CreateModelInput<M> {
	/// Build a model or return a client-visible validation failure.
	fn build(self) -> Result<M, ServerFnSetError>;
}

/// Applies a full-update DTO to an existing model.
#[cfg(all(native, feature = "model-server-fnset"))]
pub trait UpdateModelInput<M> {
	/// Apply the update to the model.
	fn apply(self, model: &mut M) -> Result<(), ServerFnSetError>;
}

/// Applies a partial-update DTO to an existing model.
#[cfg(all(native, feature = "model-server-fnset"))]
pub trait PatchModelInput<M> {
	/// Apply the patch to the model.
	fn apply_patch(self, model: &mut M) -> Result<(), ServerFnSetError>;
}

/// Native persistence contract for one model server function resource.
#[cfg(all(native, feature = "model-server-fnset"))]
#[async_trait::async_trait]
pub trait ModelServerFnResource: ServerFnResource + Send + Sync + Sized {
	/// ORM model persisted by this resource.
	type Model: reinhardt_db::orm::Model + 'static;
	/// Explicit access policy for every generated action.
	type Policy: super::ServerFnSetPolicy<Self>;
	/// Stable public resource name used in client-visible errors.
	const PUBLIC_NAME: &'static str = "resource";

	/// Return typed proof of the unique field used for detail lookups.
	fn lookup_field() -> reinhardt_db::orm::UniqueFieldRef<Self::Model, Self::Lookup>;

	/// Return the unscoped base queryset for this resource.
	fn base_queryset() -> QuerySet<Self::Model> {
		Self::Model::objects().all()
	}

	/// Apply resource-specific typed filters and ordering.
	fn apply_list_query(
		queryset: QuerySet<Self::Model>,
		_query: &Self::ListQuery,
	) -> Result<QuerySet<Self::Model>, ServerFnSetError> {
		Ok(queryset)
	}

	/// Convert a persisted model into its wire representation.
	///
	/// Read actions pass `None`. Mutation actions pass their active transaction
	/// executor so conversion cannot require a fresh database connection.
	async fn to_read(
		model: &Self::Model,
		executor: Option<&mut dyn TransactionExecutor>,
	) -> Result<Self::Read, ServerFnSetError>;

	/// Validate a create input inside the active mutation transaction.
	async fn validate_create(
		_input: &Self::Create,
		_executor: &mut dyn TransactionExecutor,
	) -> Result<(), ServerFnSetError> {
		Ok(())
	}

	/// Validate a full update inside the active mutation transaction.
	async fn validate_update(
		_input: &Self::Update,
		_object: &Self::Model,
		_executor: &mut dyn TransactionExecutor,
	) -> Result<(), ServerFnSetError> {
		Ok(())
	}

	/// Validate a partial update inside the active mutation transaction.
	async fn validate_patch(
		_input: &Self::Patch,
		_object: &Self::Model,
		_executor: &mut dyn TransactionExecutor,
	) -> Result<(), ServerFnSetError> {
		Ok(())
	}

	/// Build and persist a new model through the active transaction executor.
	async fn perform_create(
		input: Self::Create,
		executor: &mut dyn TransactionExecutor,
	) -> Result<Self::Model, ServerFnSetError>
	where
		Self::Create: CreateModelInput<Self::Model> + Send,
	{
		let mut object = input.build()?;
		object.save_with_executor(executor).await.map_err(|error| {
			tracing::error!(%error, "model server function create failed");
			ServerFnSetError::Internal
		})?;
		Ok(object)
	}

	/// Apply and persist a full update through the active transaction executor.
	async fn perform_update(
		input: Self::Update,
		object: &mut Self::Model,
		executor: &mut dyn TransactionExecutor,
	) -> Result<(), ServerFnSetError>
	where
		Self::Update: UpdateModelInput<Self::Model> + Send,
	{
		input.apply(object)?;
		object.save_with_executor(executor).await.map_err(|error| {
			tracing::error!(%error, "model server function update failed");
			ServerFnSetError::Internal
		})
	}

	/// Apply and persist a patch through the active transaction executor.
	async fn perform_patch(
		input: Self::Patch,
		object: &mut Self::Model,
		executor: &mut dyn TransactionExecutor,
	) -> Result<(), ServerFnSetError>
	where
		Self::Patch: PatchModelInput<Self::Model> + Send,
	{
		input.apply_patch(object)?;
		object.save_with_executor(executor).await.map_err(|error| {
			tracing::error!(%error, "model server function patch failed");
			ServerFnSetError::Internal
		})
	}

	/// Delete a model through the active transaction executor.
	async fn perform_destroy(
		object: &Self::Model,
		executor: &mut dyn TransactionExecutor,
	) -> Result<(), ServerFnSetError> {
		object
			.delete_with_executor(executor)
			.await
			.map_err(|error| {
				tracing::error!(%error, "model server function destroy failed");
				ServerFnSetError::Internal
			})
	}
}
