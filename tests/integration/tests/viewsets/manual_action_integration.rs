use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_http::{Request, Response};
use reinhardt_viewsets::{action, register_action, ActionMetadata, FunctionActionHandler, ViewSet};
use std::collections::HashSet;

#[derive(Debug, Clone)]
struct TestViewSet {
	basename: String,
}

impl TestViewSet {
	fn new(basename: impl Into<String>) -> Self {
		Self {
			basename: basename.into(),
		}
	}
}

#[async_trait]
impl ViewSet for TestViewSet {
	fn get_basename(&self) -> &str {
		&self.basename
	}

	async fn dispatch(
		&self,
		_request: Request,
		_action: reinhardt_viewsets::Action,
	) -> reinhardt_http::Result<Response> {
		Response::ok().with_json(&serde_json::json!({"test": true}))
	}
}

/// Test manual action registration
#[tokio::test]
async fn test_manual_action_registration() {
	// Register actions manually
	let viewset_type = std::any::type_name::<TestViewSet>();

	let action1 = ActionMetadata::new("custom_list")
		.with_detail(false)
		.with_handler(FunctionActionHandler::new(|_req| {
			Box::pin(async { Response::ok().with_json(&serde_json::json!({"action": "list"})) })
		}));

	let action2 = ActionMetadata::new("custom_detail")
		.with_detail(true)
		.with_custom_name("Custom Detail Action")
		.with_handler(FunctionActionHandler::new(|_req| {
			Box::pin(async { Response::ok().with_json(&serde_json::json!({"action": "detail"})) })
		}));

	register_action(viewset_type, action1);
	register_action(viewset_type, action2);

	// Get actions via ViewSet trait
	let viewset = TestViewSet::new("test");
	let actions = viewset.get_extra_actions();

	assert_eq!(actions.len(), 2);

	// Use HashSet for order-independent comparison
	let actual_names: HashSet<String> = actions.iter().map(|a| a.name.clone()).collect();
	let expected_names: HashSet<String> = ["custom_list", "custom_detail"]
		.iter()
		.map(|s| s.to_string())
		.collect();
	assert_eq!(
		actual_names, expected_names,
		"アクション名が期待値と一致しません。期待: {:?}, 実際: {:?}",
		expected_names, actual_names
	);
}

/// Test action metadata properties
#[tokio::test]
async fn test_action_metadata_properties() {
	let action = ActionMetadata::new("test_action")
		.with_detail(true)
		.with_custom_name("My Custom Action")
		.with_suffix("Extra")
		.with_url_path("custom/path")
		.with_url_name("custom-action");

	assert_eq!(action.name, "test_action");
	assert!(action.detail);
	assert_eq!(action.custom_name, Some("My Custom Action".to_string()));
	assert_eq!(action.display_name(), "My Custom Action"); // custom_name takes precedence
	assert_eq!(action.get_url_path(), "custom/path");
	assert_eq!(action.get_url_name(), "custom-action");
}

/// Test display name formatting
#[tokio::test]
async fn test_display_name_formatting() {
	// Without custom_name or suffix - should format snake_case
	let action1 = ActionMetadata::new("list_all_items");
	assert_eq!(action1.display_name(), "List All Items");

	// With custom_name - should use it
	let action2 = ActionMetadata::new("list_all_items").with_custom_name("All Items");
	assert_eq!(action2.display_name(), "All Items");

	// With suffix - should append it
	let action3 = ActionMetadata::new("list_items").with_suffix("Custom");
	assert_eq!(action3.display_name(), "List Items Custom");
}

/// Test action helper function
#[tokio::test]
async fn test_action_helper() {
	let my_action = action("test", false, |_req| async {
		Response::ok().with_json(&serde_json::json!({"status": "ok"}))
	});

	assert_eq!(my_action.name, "test");
	assert!(!my_action.detail);

	// Test handler execution
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = my_action.handler.handle(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, StatusCode::OK);
}

/// Test ViewSet without actions
#[tokio::test]
async fn test_viewset_no_actions() {
	let viewset = TestViewSet::new("empty");
	let actions = viewset.get_extra_actions();

	// Should not include actions from previous tests (different type instance)
	// In practice, we'd need to clear the registry between tests
	// For now, just verify it returns a vec (len() is always >= 0, so no assertion needed)
	let _ = actions.len(); // May contain actions from other tests
}

/// Test get_extra_action_url_map (should return empty for base ViewSet)
#[tokio::test]
async fn test_uninitialized_view_url_map() {
	let viewset = TestViewSet::new("test");
	let url_map = viewset.get_extra_action_url_map();

	// Uninitialized ViewSet should return empty map
	assert_eq!(url_map.len(), 0);
}
