use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use reinhardt_db::orm::{
	DatabaseConnection, FilterValue, Model, TransactionExecutor, TransactionScope,
};

use super::{
	CollectionActionContext, CollectionReadActionContext, CreateActionContext, DetailActionContext,
	DetailReadActionContext,
};
use super::{
	CreateModelInput, ModelServerFnResource, Page, PatchModelInput, ServerFnListQuery,
	ServerFnSetAction, ServerFnSetError, ServerFnSetPolicy, UpdateModelInput,
};

type Principal<R> = <<R as ModelServerFnResource>::Policy as ServerFnSetPolicy<R>>::Principal;

/// Policy-aware runtime for the six standard model server function actions.
pub struct ModelServerFnSet<R>(PhantomData<fn() -> R>);

impl<R> Default for ModelServerFnSet<R> {
	fn default() -> Self {
		Self::new()
	}
}

impl<R> ModelServerFnSet<R> {
	/// Construct a zero-sized model server function set runtime.
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

struct MutationTransaction {
	scope: TransactionScope,
}

impl MutationTransaction {
	async fn begin(connection: &DatabaseConnection) -> Result<Self, ServerFnSetError> {
		let scope = TransactionScope::begin(connection)
			.await
			.map_err(internal_error)?;
		Ok(Self { scope })
	}

	fn executor_mut(
		&mut self,
	) -> Result<&mut (dyn TransactionExecutor + 'static), ServerFnSetError> {
		self.scope.executor_mut().map_err(internal_error)
	}

	async fn complete<T>(self, result: Result<T, ServerFnSetError>) -> Result<T, ServerFnSetError> {
		match result {
			Ok(value) => {
				self.scope.commit().await.map_err(internal_error)?;
				Ok(value)
			}
			Err(error) => match self.scope.rollback().await {
				Ok(()) => Err(error),
				Err(rollback_error) => {
					tracing::error!(%rollback_error, "model server function transaction rollback failed");
					Err(ServerFnSetError::Internal)
				}
			},
		}
	}
}

fn internal_error(error: impl std::fmt::Display) -> ServerFnSetError {
	tracing::error!(%error, "model server function runtime failed");
	ServerFnSetError::Internal
}

fn map_detail<R: ModelServerFnResource>(
	mut rows: Vec<R::Model>,
) -> Result<R::Model, ServerFnSetError> {
	match rows.len() {
		0 => Err(ServerFnSetError::NotFound {
			resource: R::PUBLIC_NAME.to_owned(),
		}),
		1 => Ok(rows.pop().expect("one-row branch must contain one model")),
		_ => {
			tracing::error!(
				resource = R::Model::table_name(),
				"unique model lookup returned multiple rows"
			);
			Err(ServerFnSetError::Internal)
		}
	}
}

impl<R> ModelServerFnSet<R>
where
	R: ModelServerFnResource,
	R::Lookup: Into<FilterValue> + Send,
	R::Create: CreateModelInput<R::Model> + Send,
	R::Update: UpdateModelInput<R::Model> + Send,
	R::Patch: PatchModelInput<R::Model> + Send,
{
	/// Execute the non-transactional list pipeline through an explicit connection.
	pub async fn list(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		query: R::ListQuery,
	) -> Result<Page<R::Read>, ServerFnSetError> {
		let page = query.page_request().validate()?;
		<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
			principal,
			ServerFnSetAction::List,
			None,
		)
		.await?;
		let queryset = R::base_queryset();
		let queryset =
			<R::Policy as ServerFnSetPolicy<R>>::scope_query(principal, queryset, None).await?;
		let queryset = R::apply_list_query(queryset, &query)?;
		let total = queryset
			.count_with_db(connection)
			.await
			.map_err(internal_error)?;
		let offset = usize::try_from(page.offset).map_err(|error| {
			tracing::error!(%error, offset = page.offset, "pagination offset is not representable");
			ServerFnSetError::Internal
		})?;
		let models = queryset
			.offset(offset)
			.limit(page.limit as usize)
			.all_with_db(connection)
			.await
			.map_err(internal_error)?;
		let mut items = Vec::with_capacity(models.len());
		for model in &models {
			items.push(R::to_read(model, None).await?);
		}
		Ok(Page {
			items,
			total,
			limit: page.limit,
			offset: page.offset,
		})
	}

	/// Execute the non-transactional detail read pipeline.
	pub async fn retrieve(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		lookup: R::Lookup,
	) -> Result<R::Read, ServerFnSetError> {
		<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
			principal,
			ServerFnSetAction::Retrieve,
			None,
		)
		.await?;
		let rows = R::base_queryset()
			.filter(R::lookup_field().eq(lookup))
			.one_with_db(connection)
			.await
			.map_err(internal_error)?;
		let object = map_detail::<R>(rows)?;
		<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
			principal,
			ServerFnSetAction::Retrieve,
			&object,
			None,
		)
		.await?;
		R::to_read(&object, None).await
	}

	/// Execute the transactional create pipeline.
	///
	/// The created object is authorized before it is converted and committed so
	/// policies can enforce permissions that depend on submitted object fields.
	pub async fn create(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		input: R::Create,
	) -> Result<R::Read, ServerFnSetError> {
		let mut transaction = MutationTransaction::begin(connection).await?;
		let result = async {
			<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
				principal,
				ServerFnSetAction::Create,
				Some(transaction.executor_mut()?),
			)
			.await?;
			R::validate_create(&input, transaction.executor_mut()?).await?;
			let object = R::perform_create(input, transaction.executor_mut()?).await?;
			<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
				principal,
				ServerFnSetAction::Create,
				&object,
				Some(transaction.executor_mut()?),
			)
			.await?;
			R::to_read(&object, Some(transaction.executor_mut()?)).await
		}
		.await;
		transaction.complete(result).await
	}

	/// Execute the transactional full-update pipeline.
	pub async fn update(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		lookup: R::Lookup,
		input: R::Update,
	) -> Result<R::Read, ServerFnSetError> {
		let mut transaction = MutationTransaction::begin(connection).await?;
		let result = async {
			<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
				principal,
				ServerFnSetAction::Update,
				Some(transaction.executor_mut()?),
			)
			.await?;
			let mut object = Self::lookup_in_transaction(&mut transaction, lookup).await?;
			<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
				principal,
				ServerFnSetAction::Update,
				&object,
				Some(transaction.executor_mut()?),
			)
			.await?;
			R::validate_update(&input, &object, transaction.executor_mut()?).await?;
			R::perform_update(input, &mut object, transaction.executor_mut()?).await?;
			<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
				principal,
				ServerFnSetAction::Update,
				&object,
				Some(transaction.executor_mut()?),
			)
			.await?;
			R::to_read(&object, Some(transaction.executor_mut()?)).await
		}
		.await;
		transaction.complete(result).await
	}

	/// Execute the transactional partial-update pipeline.
	pub async fn partial_update(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		lookup: R::Lookup,
		input: R::Patch,
	) -> Result<R::Read, ServerFnSetError> {
		let mut transaction = MutationTransaction::begin(connection).await?;
		let result = async {
			<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
				principal,
				ServerFnSetAction::PartialUpdate,
				Some(transaction.executor_mut()?),
			)
			.await?;
			let mut object = Self::lookup_in_transaction(&mut transaction, lookup).await?;
			<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
				principal,
				ServerFnSetAction::PartialUpdate,
				&object,
				Some(transaction.executor_mut()?),
			)
			.await?;
			R::validate_patch(&input, &object, transaction.executor_mut()?).await?;
			R::perform_patch(input, &mut object, transaction.executor_mut()?).await?;
			<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
				principal,
				ServerFnSetAction::PartialUpdate,
				&object,
				Some(transaction.executor_mut()?),
			)
			.await?;
			R::to_read(&object, Some(transaction.executor_mut()?)).await
		}
		.await;
		transaction.complete(result).await
	}

	/// Execute the transactional destroy pipeline.
	pub async fn destroy(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		lookup: R::Lookup,
	) -> Result<(), ServerFnSetError> {
		let mut transaction = MutationTransaction::begin(connection).await?;
		let result = async {
			<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
				principal,
				ServerFnSetAction::Destroy,
				Some(transaction.executor_mut()?),
			)
			.await?;
			let object = Self::lookup_in_transaction(&mut transaction, lookup).await?;
			<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
				principal,
				ServerFnSetAction::Destroy,
				&object,
				Some(transaction.executor_mut()?),
			)
			.await?;
			R::perform_destroy(&object, transaction.executor_mut()?).await
		}
		.await;
		transaction.complete(result).await
	}

	async fn lookup_in_transaction(
		transaction: &mut MutationTransaction,
		lookup: R::Lookup,
	) -> Result<R::Model, ServerFnSetError> {
		let rows = R::base_queryset()
			.filter(R::lookup_field().eq(lookup))
			.one_with_executor(transaction.executor_mut()?)
			.await
			.map_err(internal_error)?;
		map_detail::<R>(rows)
	}

	/// Execute a transaction-bound detail override or custom action.
	pub async fn transactional_detail_action<T, F>(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		lookup: R::Lookup,
		action: ServerFnSetAction,
		callback: F,
	) -> Result<T, ServerFnSetError>
	where
		F: for<'a> FnOnce(
			DetailActionContext<'a, R>,
		)
			-> Pin<Box<dyn Future<Output = Result<T, ServerFnSetError>> + Send + 'a>>,
	{
		let mut transaction = MutationTransaction::begin(connection).await?;
		let result = async {
			<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
				principal,
				action,
				Some(transaction.executor_mut()?),
			)
			.await?;
			let object = Self::lookup_in_transaction(&mut transaction, lookup).await?;
			<R::Policy as ServerFnSetPolicy<R>>::authorize_object(
				principal,
				action,
				&object,
				Some(transaction.executor_mut()?),
			)
			.await?;
			callback(DetailActionContext::new(
				object,
				transaction.executor_mut()?,
			))
			.await
		}
		.await;
		transaction.complete(result).await
	}

	/// Execute a transaction-bound collection override or custom action.
	pub async fn transactional_collection_action<T, F>(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		action: ServerFnSetAction,
		callback: F,
	) -> Result<T, ServerFnSetError>
	where
		F: for<'a> FnOnce(
			CollectionActionContext<'a, R>,
		)
			-> Pin<Box<dyn Future<Output = Result<T, ServerFnSetError>> + Send + 'a>>,
	{
		let mut transaction = MutationTransaction::begin(connection).await?;
		let result = async {
			<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
				principal,
				action,
				Some(transaction.executor_mut()?),
			)
			.await?;
			let queryset = <R::Policy as ServerFnSetPolicy<R>>::scope_query(
				principal,
				R::base_queryset(),
				Some(transaction.executor_mut()?),
			)
			.await?;
			callback(CollectionActionContext::new(
				queryset,
				transaction.executor_mut()?,
			))
			.await
		}
		.await;
		transaction.complete(result).await
	}

	/// Execute a standard create override without an existing-object queryset.
	pub async fn transactional_create_action<T, F>(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		callback: F,
	) -> Result<T, ServerFnSetError>
	where
		F: for<'a> FnOnce(
			CreateActionContext<'a, R>,
		)
			-> Pin<Box<dyn Future<Output = Result<T, ServerFnSetError>> + Send + 'a>>,
	{
		let mut transaction = MutationTransaction::begin(connection).await?;
		let result = async {
			<R::Policy as ServerFnSetPolicy<R>>::authorize_action(
				principal,
				ServerFnSetAction::Create,
				Some(transaction.executor_mut()?),
			)
			.await?;
			callback(CreateActionContext::new(transaction.executor_mut()?)).await
		}
		.await;
		transaction.complete(result).await
	}

	/// Execute a non-transactional detail custom action.
	pub async fn read_detail_action<T, F>(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		lookup: R::Lookup,
		action: ServerFnSetAction,
		callback: F,
	) -> Result<T, ServerFnSetError>
	where
		F: for<'a> FnOnce(
			DetailReadActionContext<'a, R>,
		)
			-> Pin<Box<dyn Future<Output = Result<T, ServerFnSetError>> + Send + 'a>>,
	{
		<R::Policy as ServerFnSetPolicy<R>>::authorize_action(principal, action, None).await?;
		let rows = R::base_queryset()
			.filter(R::lookup_field().eq(lookup))
			.one_with_db(connection)
			.await
			.map_err(internal_error)?;
		let object = map_detail::<R>(rows)?;
		<R::Policy as ServerFnSetPolicy<R>>::authorize_object(principal, action, &object, None)
			.await?;
		callback(DetailReadActionContext::new(object, connection)).await
	}

	/// Execute a non-transactional collection custom action.
	pub async fn read_collection_action<T, F>(
		principal: &Principal<R>,
		connection: &DatabaseConnection,
		action: ServerFnSetAction,
		callback: F,
	) -> Result<T, ServerFnSetError>
	where
		F: for<'a> FnOnce(
			CollectionReadActionContext<'a, R>,
		)
			-> Pin<Box<dyn Future<Output = Result<T, ServerFnSetError>> + Send + 'a>>,
	{
		<R::Policy as ServerFnSetPolicy<R>>::authorize_action(principal, action, None).await?;
		let queryset =
			<R::Policy as ServerFnSetPolicy<R>>::scope_query(principal, R::base_queryset(), None)
				.await?;
		callback(CollectionReadActionContext::new(queryset, connection)).await
	}
}
