//! ViewSets for Reinhardt framework
//!
//! This crate provides ViewSet functionality for building REST APIs with automatic
//! routing, pagination, filtering, and caching support.

pub mod actions;
pub mod batch_operations;
pub mod builder;
pub mod cached;
pub mod di_support;
pub mod filtering_support;
pub mod handler;
pub mod metadata;
pub mod middleware;
pub mod mixins;
pub mod nested_resources;
pub mod pagination_support;
pub mod registry;
pub mod schema_metadata;
pub mod test_viewset;
pub mod viewset;

pub use actions::{Action, ActionType};
pub use batch_operations::{
	BatchOperation, BatchOperationResult, BatchProcessor, BatchRequest, BatchResponse,
	BatchStatistics,
};
pub use builder::ViewSetBuilder;
pub use cached::{CacheConfig, CachedResponse, CachedViewSet, CachedViewSetTrait};
pub use di_support::{DatabaseConnection, DiViewSet, ViewSetFactory};
pub use filtering_support::{FilterConfig, FilterableViewSet, InMemoryFilter, OrderingConfig};
pub use handler::{ModelViewSetHandler, ViewError, ViewSetHandler};
pub use metadata::{ActionHandler, ActionMetadata, ActionRegistryEntry, FunctionActionHandler};
pub use middleware::{
	AuthenticationMiddleware, CompositeMiddleware, PermissionMiddleware, ViewSetMiddleware,
};
pub use mixins::{
	BulkCreateMixin, BulkDeleteMixin, BulkOperationsMixin, BulkUpdateMixin, CreateMixin,
	DestroyMixin, ListMixin, RetrieveMixin, UpdateMixin,
};
pub use nested_resources::{
	NestedResource, NestedResourcePath, NestedViewSet, nested_detail_url, nested_url,
};
pub use pagination_support::{PaginatedViewSet, PaginationConfig};
pub use registry::{action, get_registered_actions, register_action};
pub use schema_metadata::{FieldSchema, ModelSchema, RequestSchema, ResponseSchema, ViewSetSchema};
pub use test_viewset::{SimpleViewSet, TestViewSet};
pub use viewset::{GenericViewSet, ModelViewSet, ReadOnlyModelViewSet, ViewSet};

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
	use reinhardt_apps::Request;

	#[allow(dead_code)]
	#[derive(Debug, Clone)]
	struct TestModel {
		id: i64,
		name: String,
	}

	#[derive(Debug, Clone)]
	struct TestSerializer;

	#[tokio::test]
	async fn test_viewset_get_basename() {
		let viewset = GenericViewSet::new("test", ());
		assert_eq!(viewset.get_basename(), "test");
	}

	#[tokio::test]
	async fn test_model_viewset_list_action() {
		let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("users");
		let request = Request::new(
			Method::GET,
			Uri::from_static("/users/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);
		let action = Action::list();

		let response = viewset.dispatch(request, action).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
	}

	#[tokio::test]
	async fn test_model_viewset_retrieve_action() {
		let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("users");
		let request = Request::new(
			Method::GET,
			Uri::from_static("/users/1/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);
		let action = Action::retrieve();

		let response = viewset.dispatch(request, action).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
	}

	#[tokio::test]
	async fn test_model_viewset_create_action() {
		let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("users");
		let request = Request::new(
			Method::POST,
			Uri::from_static("/users/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::from(r#"{"name": "test"}"#),
		);
		let action = Action::create();

		let response = viewset.dispatch(request, action).await.unwrap();

		assert_eq!(response.status, StatusCode::CREATED);
	}

	#[tokio::test]
	async fn test_model_viewset_update_action() {
		let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("users");
		let request = Request::new(
			Method::PUT,
			Uri::from_static("/users/1/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::from(r#"{"name": "updated"}"#),
		);
		let action = Action::update();

		let response = viewset.dispatch(request, action).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
	}

	#[tokio::test]
	async fn test_model_viewset_destroy_action() {
		let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("users");
		let request = Request::new(
			Method::DELETE,
			Uri::from_static("/users/1/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);
		let action = Action::destroy();

		let response = viewset.dispatch(request, action).await.unwrap();

		assert_eq!(response.status, StatusCode::NO_CONTENT);
	}

	#[tokio::test]
	async fn test_readonly_viewset_list_allowed() {
		let viewset: ReadOnlyModelViewSet<TestModel, TestSerializer> =
			ReadOnlyModelViewSet::new("posts");
		let request = Request::new(
			Method::GET,
			Uri::from_static("/posts/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);
		let action = Action::list();

		let response = viewset.dispatch(request, action).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
	}

	#[tokio::test]
	async fn test_readonly_viewset_retrieve_allowed() {
		let viewset: ReadOnlyModelViewSet<TestModel, TestSerializer> =
			ReadOnlyModelViewSet::new("posts");
		let request = Request::new(
			Method::GET,
			Uri::from_static("/posts/1/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);
		let action = Action::retrieve();

		let response = viewset.dispatch(request, action).await.unwrap();

		assert_eq!(response.status, StatusCode::OK);
	}

	#[tokio::test]
	async fn test_readonly_viewset_create_denied() {
		let viewset: ReadOnlyModelViewSet<TestModel, TestSerializer> =
			ReadOnlyModelViewSet::new("posts");
		let request = Request::new(
			Method::POST,
			Uri::from_static("/posts/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::from(r#"{"title": "test"}"#),
		);
		let action = Action::create();

		let result = viewset.dispatch(request, action).await;

		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_readonly_viewset_delete_denied() {
		let viewset: ReadOnlyModelViewSet<TestModel, TestSerializer> =
			ReadOnlyModelViewSet::new("posts");
		let request = Request::new(
			Method::DELETE,
			Uri::from_static("/posts/1/"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		);
		let action = Action::destroy();

		let result = viewset.dispatch(request, action).await;

		assert!(result.is_err());
	}
}
