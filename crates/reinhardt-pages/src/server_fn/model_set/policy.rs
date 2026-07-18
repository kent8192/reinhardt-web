use reinhardt_db::orm::{QuerySet, TransactionExecutor};
use reinhardt_di::params::{FromRequest, ParamContext, ParamResult};
use reinhardt_http::Request;

use super::{ModelServerFnResource, ServerFnSetError};

/// Extractor-only wrapper for the principal selected by a resource policy.
pub struct PolicyPrincipal<R: ModelServerFnResource>(
	pub <<R as ModelServerFnResource>::Policy as ServerFnSetPolicy<R>>::Principal,
);

#[async_trait::async_trait]
impl<R> FromRequest for PolicyPrincipal<R>
where
	R: ModelServerFnResource + 'static,
{
	async fn from_request(request: &Request, context: &ParamContext) -> ParamResult<Self> {
		<R::Policy as ServerFnSetPolicy<R>>::Principal::from_request(request, context)
			.await
			.map(Self)
	}
}

/// Logical action being authorized by a model server function policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerFnSetAction {
	/// List resources.
	List,
	/// Retrieve one resource.
	Retrieve,
	/// Create one resource.
	Create,
	/// Fully update one resource.
	Update,
	/// Partially update one resource.
	PartialUpdate,
	/// Destroy one resource.
	Destroy,
	/// Run a named custom action.
	Custom(&'static str),
}

/// Authorization and query-scoping contract for a model server function set.
#[async_trait::async_trait]
pub trait ServerFnSetPolicy<R: ModelServerFnResource>: Send + Sync + 'static {
	/// Principal extracted uniformly from the incoming request.
	///
	/// Implementations should return [`reinhardt_di::params::ParamError::Authentication`]
	/// for a missing or invalid identity, and
	/// [`reinhardt_di::params::ParamError::Internal`] for provider or dependency
	/// failures. Generated handlers preserve those status semantics while
	/// removing the error details from client-visible responses.
	type Principal: FromRequest + Sync + 'static;

	/// Authorize an action before resource access.
	///
	/// Read actions pass `None`. Mutation actions pass the active transaction
	/// executor, and every policy and resource hook receives that same identity.
	async fn authorize_action(
		principal: &Self::Principal,
		action: ServerFnSetAction,
		executor: Option<&mut dyn TransactionExecutor>,
	) -> Result<(), ServerFnSetError>;

	/// Scope the resource queryset to objects visible to the principal.
	///
	/// Standard collection reads are non-transactional and pass `None`.
	async fn scope_query(
		principal: &Self::Principal,
		query: QuerySet<R::Model>,
		executor: Option<&mut dyn TransactionExecutor>,
	) -> Result<QuerySet<R::Model>, ServerFnSetError>;

	/// Authorize an action against a loaded object.
	///
	/// Mutation actions pass their active transaction executor; detail reads pass
	/// `None` and remain non-transactional.
	async fn authorize_object(
		principal: &Self::Principal,
		action: ServerFnSetAction,
		object: &R::Model,
		executor: Option<&mut dyn TransactionExecutor>,
	) -> Result<(), ServerFnSetError>;
}

/// Principal used by the explicit unrestricted policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllowAllPrincipal;

#[async_trait::async_trait]
impl FromRequest for AllowAllPrincipal {
	async fn from_request(_request: &Request, _context: &ParamContext) -> ParamResult<Self> {
		Ok(Self)
	}
}

/// Explicit policy that authorizes every action and object.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAllPolicy;

#[async_trait::async_trait]
impl<R> ServerFnSetPolicy<R> for AllowAllPolicy
where
	R: ModelServerFnResource,
{
	type Principal = AllowAllPrincipal;

	async fn authorize_action(
		_principal: &Self::Principal,
		_action: ServerFnSetAction,
		_executor: Option<&mut dyn TransactionExecutor>,
	) -> Result<(), ServerFnSetError> {
		Ok(())
	}

	async fn scope_query(
		_principal: &Self::Principal,
		query: QuerySet<R::Model>,
		_executor: Option<&mut dyn TransactionExecutor>,
	) -> Result<QuerySet<R::Model>, ServerFnSetError> {
		Ok(query)
	}

	async fn authorize_object(
		_principal: &Self::Principal,
		_action: ServerFnSetAction,
		_object: &R::Model,
		_executor: Option<&mut dyn TransactionExecutor>,
	) -> Result<(), ServerFnSetError> {
		Ok(())
	}
}
