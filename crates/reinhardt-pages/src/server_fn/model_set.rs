//! Contracts for typed model-backed server function sets.

mod error;
mod pagination;
mod resource;

#[cfg(all(native, feature = "model-server-fnset"))]
mod context;
#[cfg(all(native, feature = "model-server-fnset"))]
mod policy;
#[cfg(all(native, feature = "model-server-fnset"))]
mod runtime;

pub use error::{FieldError, FieldErrors, ServerFnSetError};
pub use pagination::{Page, PageRequest, ServerFnListQuery, ValidatedPageRequest};
pub use resource::ServerFnResource;

#[cfg(all(native, feature = "model-server-fnset"))]
pub use context::{
	CollectionActionContext, CollectionReadActionContext, CreateActionContext, DetailActionContext,
	DetailReadActionContext,
};
#[cfg(all(native, feature = "model-server-fnset"))]
pub use policy::{
	AllowAllPolicy, AllowAllPrincipal, PolicyPrincipal, ServerFnSetAction, ServerFnSetPolicy,
};
#[cfg(all(native, feature = "model-server-fnset"))]
pub use resource::{CreateModelInput, ModelServerFnResource, PatchModelInput, UpdateModelInput};
#[cfg(all(native, feature = "model-server-fnset"))]
pub use runtime::ModelServerFnSet;

/// Hidden fn/impl linkage contract emitted by `server_fnset`.
#[doc(hidden)]
pub trait ModelServerFnSetLink {
	/// Resource selected by the fn-form declaration.
	type Resource: ServerFnResource;
	/// Stable set name.
	const NAME: &'static str;
}
