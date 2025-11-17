//! Integration tests for filtering and ordering support in ViewSets

use bytes::Bytes;
use hyper::{HeaderMap, Method, Version};
use reinhardt_http::Request;
use reinhardt_viewsets::{
	FilterConfig, FilterableViewSet, ModelViewSet, OrderingConfig, ReadOnlyModelViewSet,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestModel {
	id: i64,
	name: String,
	status: String,
	category: String,
	created_at: i64,
}

#[derive(Debug, Clone)]
struct TestSerializer;

#[tokio::test]
async fn test_viewset_default_no_filter_config() {
	let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("items");
	let config = viewset.get_filter_config();
	assert!(config.is_none());
}

#[tokio::test]
async fn test_viewset_with_filters() {
	let filter_config = FilterConfig::new()
		.with_filterable_fields(vec!["status", "category"])
		.with_search_fields(vec!["name"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_filters(filter_config.clone());

	let config = viewset.get_filter_config();
	let config = config.unwrap();
	assert_eq!(config.filterable_fields.len(), 2);

	use std::collections::HashSet;
	assert_eq!(
		config.filterable_fields.iter().collect::<HashSet<_>>(),
		HashSet::from([&"status".to_string(), &"category".to_string()]),
		"Filterable fields mismatch. Expected fields: {:?}, Got: {:?}",
		["status", "category"],
		config.filterable_fields
	);

	assert_eq!(config.search_fields.len(), 1);
	assert_eq!(
		config.search_fields.iter().collect::<HashSet<_>>(),
		HashSet::from([&"name".to_string()]),
		"Search fields mismatch. Expected fields: {:?}, Got: {:?}",
		["name"],
		config.search_fields
	);
}

#[tokio::test]
async fn test_extract_filters_from_request() {
	let filter_config = FilterConfig::new()
		.with_filterable_fields(vec!["status", "category"])
		.with_search_fields(vec!["name"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_filters(filter_config);

	let request = Request::builder()
		.method(Method::GET)
		.uri("/items/?status=active&category=tech&invalid=ignored")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let filters = viewset.extract_filters(&request);
	assert_eq!(filters.len(), 2);
	assert_eq!(filters.get("status"), Some(&"active".to_string()));
	assert_eq!(filters.get("category"), Some(&"tech".to_string()));
	assert!(!filters.contains_key("invalid"));
}

#[tokio::test]
async fn test_extract_search_from_request() {
	let filter_config = FilterConfig::new().with_search_fields(vec!["name", "description"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_filters(filter_config);

	let request = Request::builder()
		.method(Method::GET)
		.uri("/items/?search=rust%20programming")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let search = viewset.extract_search(&request);
	assert_eq!(search, Some("rust programming".to_string()));
}

#[tokio::test]
async fn test_viewset_with_ordering() {
	let ordering_config = OrderingConfig::new()
		.with_ordering_fields(vec!["created_at", "name"])
		.with_default_ordering(vec!["-created_at"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_ordering(ordering_config.clone());

	let config = viewset.get_ordering_config();
	let config = config.unwrap();
	assert_eq!(config.ordering_fields.len(), 2);

	use std::collections::HashSet;
	assert_eq!(
		config.ordering_fields.iter().collect::<HashSet<_>>(),
		HashSet::from([&"created_at".to_string(), &"name".to_string()]),
		"Ordering fields mismatch. Expected fields: {:?}, Got: {:?}",
		["created_at", "name"],
		config.ordering_fields
	);

	assert_eq!(config.default_ordering.len(), 1);
	assert_eq!(config.default_ordering[0], "-created_at");
}

#[tokio::test]
async fn test_extract_ordering_from_request() {
	let ordering_config = OrderingConfig::new()
		.with_ordering_fields(vec!["created_at", "name", "id"])
		.with_default_ordering(vec!["-created_at"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_ordering(ordering_config);

	let request = Request::builder()
		.method(Method::GET)
		.uri("/items/?ordering=name,-created_at")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ordering = viewset.extract_ordering(&request);
	assert_eq!(ordering.len(), 2);
	assert_eq!(ordering[0], "name");
	assert_eq!(ordering[1], "-created_at");
}

#[tokio::test]
async fn test_extract_ordering_with_validation() {
	let ordering_config = OrderingConfig::new()
		.with_ordering_fields(vec!["created_at", "name"])
		.with_default_ordering(vec!["-created_at"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_ordering(ordering_config);

	// Request invalid field
	let request = Request::builder()
		.method(Method::GET)
		.uri("/items/?ordering=invalid_field")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ordering = viewset.extract_ordering(&request);
	// Should fall back to default ordering when all requested fields are invalid
	assert_eq!(ordering.len(), 1);
	assert_eq!(ordering[0], "-created_at");
}

#[tokio::test]
async fn test_extract_ordering_default() {
	let ordering_config = OrderingConfig::new()
		.with_ordering_fields(vec!["created_at", "name"])
		.with_default_ordering(vec!["-created_at", "name"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_ordering(ordering_config);

	// Request without ordering parameter
	let request = Request::builder()
		.method(Method::GET)
		.uri("/items/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ordering = viewset.extract_ordering(&request);
	// Should use default ordering
	assert_eq!(ordering.len(), 2);
	assert_eq!(ordering[0], "-created_at");
	assert_eq!(ordering[1], "name");
}

#[tokio::test]
async fn test_readonly_viewset_with_filters_and_ordering() {
	let filter_config = FilterConfig::new()
		.with_filterable_fields(vec!["status"])
		.with_search_fields(vec!["name"]);

	let ordering_config = OrderingConfig::new()
		.with_ordering_fields(vec!["created_at"])
		.with_default_ordering(vec!["-created_at"]);

	let viewset: ReadOnlyModelViewSet<TestModel, TestSerializer> =
		ReadOnlyModelViewSet::new("items")
			.with_filters(filter_config)
			.with_ordering(ordering_config);

	assert!(viewset.get_filter_config().is_some());
	assert!(viewset.get_ordering_config().is_some());
}

#[tokio::test]
async fn test_builder_pattern_chaining() {
	let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("items")
		.with_filters(
			FilterConfig::new()
				.with_filterable_fields(vec!["status", "category"])
				.with_search_fields(vec!["name"]),
		)
		.with_ordering(
			OrderingConfig::new()
				.with_ordering_fields(vec!["created_at", "name"])
				.with_default_ordering(vec!["-created_at"]),
		);

	assert!(viewset.get_filter_config().is_some());
	assert!(viewset.get_ordering_config().is_some());
}

#[tokio::test]
async fn test_filter_config_case_insensitive_default() {
	let config = FilterConfig::new();
	assert!(config.case_insensitive_search);
}

#[tokio::test]
async fn test_filter_config_case_sensitive() {
	let config = FilterConfig::new().case_insensitive(false);
	assert!(!config.case_insensitive_search);
}

#[tokio::test]
async fn test_extract_filters_url_decoding() {
	let filter_config = FilterConfig::new().with_filterable_fields(vec!["name"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_filters(filter_config);

	let request = Request::builder()
		.method(Method::GET)
		.uri("/items/?name=hello%20world")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let filters = viewset.extract_filters(&request);
	assert_eq!(filters.get("name"), Some(&"hello world".to_string()));
}

#[tokio::test]
async fn test_extract_filters_skip_pagination_params() {
	let filter_config = FilterConfig::new().with_filterable_fields(vec!["status"]);

	let viewset: ModelViewSet<TestModel, TestSerializer> =
		ModelViewSet::new("items").with_filters(filter_config);

	let request = Request::builder()
		.method(Method::GET)
		.uri("/items/?status=active&page=2&page_size=10&limit=20&offset=0&cursor=abc&ordering=name&search=test")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let filters = viewset.extract_filters(&request);
	// Only status should be in filters
	assert_eq!(filters.len(), 1);
	assert_eq!(filters.get("status"), Some(&"active".to_string()));
	assert!(!filters.contains_key("page"));
	assert!(!filters.contains_key("page_size"));
	assert!(!filters.contains_key("limit"));
	assert!(!filters.contains_key("offset"));
	assert!(!filters.contains_key("cursor"));
	assert!(!filters.contains_key("ordering"));
	assert!(!filters.contains_key("search"));
}
