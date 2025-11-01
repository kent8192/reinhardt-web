use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_viewsets::viewset::ModelViewSet;
use std::collections::HashMap;
use std::sync::Arc;

// Helper function to create a test request
fn create_test_request(method: Method, path: &str) -> Request {
	let uri: Uri = path.parse().unwrap();
	let version = Version::HTTP_11;
	let headers = HeaderMap::new();
	let body = hyper::body::Bytes::new();
	Request::new(method, uri, version, headers, body)
}

#[tokio::test]
async fn test_viewset_builder_validation_empty_actions() {
	let viewset = ModelViewSet::<(), ()>::new("test");
	let builder = viewset.as_view();

	// Test that empty actions causes build to fail
	let result = builder.build();
	assert!(result.is_err());

	// Check error message without unwrapping
	match result {
		Err(e) => assert!(
			e.to_string()
				.contains("The `actions` argument must be provided")
		),
		Ok(_) => panic!("Expected error but got success"),
	}
}

#[tokio::test]
async fn test_viewset_builder_name_suffix_mutual_exclusivity() {
	let viewset = ModelViewSet::<(), ()>::new("test");
	let builder = viewset.as_view();

	// Test that providing both name and suffix fails
	let result = builder
		.with_name("test_name")
		.and_then(|b| b.with_suffix("test_suffix"));

	assert!(result.is_err());

	// Check error message without unwrapping
	match result {
		Err(e) => assert!(e.to_string().contains("received both `name` and `suffix`")),
		Ok(_) => panic!("Expected error but got success"),
	}
}

#[tokio::test]
async fn test_viewset_builder_successful_build() {
	let viewset = ModelViewSet::<(), ()>::new("test");
	let mut actions = HashMap::new();
	actions.insert(Method::GET, "list".to_string());

	let builder = viewset.as_view();
	let result = builder.with_actions(actions).build();

	assert!(result.is_ok());
	let handler = result.unwrap();

	// Test that handler is created successfully
	// Handler should be created without errors
	assert!(Arc::strong_count(&handler) > 0);
}

#[tokio::test]
async fn test_viewset_handler_attribute_tracking() {
	let viewset = ModelViewSet::<(), ()>::new("test");
	let mut actions = HashMap::new();
	actions.insert(Method::GET, "list".to_string());

	let builder = viewset.as_view();
	let handler = builder.with_actions(actions).build().unwrap();

	// Create a test request
	let request = create_test_request(Method::GET, "http://example.com/test/");

	// Handle the request - this should work without errors
	let response = handler.handle(request).await;
	assert!(response.is_ok());
}

#[tokio::test]
async fn test_viewset_handler_action_mapping() {
	let viewset = ModelViewSet::<(), ()>::new("test");
	let mut actions = HashMap::new();
	actions.insert(Method::GET, "list".to_string());
	actions.insert(Method::POST, "create".to_string());

	let builder = viewset.as_view();
	let handler = builder.with_actions(actions).build().unwrap();

	// Test GET request maps to list action
	let get_request = create_test_request(Method::GET, "http://example.com/test/");
	let response = handler.handle(get_request).await;
	assert!(response.is_ok());

	// Test POST request maps to create action
	let post_request = create_test_request(Method::POST, "http://example.com/test/");
	let response = handler.handle(post_request).await;
	assert!(response.is_ok());

	// Test unsupported method fails
	let put_request = create_test_request(Method::PUT, "http://example.com/test/");
	let response = handler.handle(put_request).await;
	assert!(response.is_err());
}

#[tokio::test]
async fn test_viewset_builder_with_name() {
	let viewset = ModelViewSet::<(), ()>::new("test");
	let mut actions = HashMap::new();
	actions.insert(Method::GET, "list".to_string());

	let builder = viewset.as_view();
	let result = builder
		.with_actions(actions)
		.with_name("test_view")
		.and_then(|b| b.build());

	assert!(result.is_ok());
}

#[tokio::test]
async fn test_viewset_builder_with_suffix() {
	let viewset = ModelViewSet::<(), ()>::new("test");
	let mut actions = HashMap::new();
	actions.insert(Method::GET, "list".to_string());

	let builder = viewset.as_view();
	let result = builder
		.with_actions(actions)
		.with_suffix("_list")
		.and_then(|b| b.build());

	assert!(result.is_ok());
}
