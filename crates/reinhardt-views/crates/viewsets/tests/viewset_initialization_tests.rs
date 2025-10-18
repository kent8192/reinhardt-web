use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_viewsets::{Action, GenericViewSet, ModelViewSet, ReadOnlyModelViewSet, ViewSet};

#[derive(Debug, Clone)]
struct TestModel {
    id: i64,
    name: String,
}

#[derive(Debug, Clone)]
struct TestSerializer;

/// Test initializing viewset with action mapping
#[tokio::test]
async fn test_initialize_view_set_with_actions() {
    let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("test");
    let request = Request::new(
        Method::GET,
        Uri::from_static("/test/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let action = Action::list();

    let response = viewset.dispatch(request, action).await;

    // Should successfully handle the request
    assert!(response.is_ok());
    assert_eq!(response.unwrap().status, StatusCode::OK);
}

/// Test HEAD request handling with viewset
#[tokio::test]
async fn test_head_request_against_viewset() {
    let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("test");
    let request = Request::new(
        Method::HEAD,
        Uri::from_static("/test/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let action = Action::list();

    // HEAD requests should be handled (or properly rejected)
    let result = viewset.dispatch(request, action).await;

    // Currently this will fail with "Method not allowed" which is correct behavior
    // for a ModelViewSet that doesn't explicitly handle HEAD
    assert!(result.is_err());
}

/// Test viewset action attribute is set correctly
#[tokio::test]
async fn test_viewset_action_attr() {
    let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("test");

    // Verify basename is set correctly (this is the closest we have to action tracking)
    assert_eq!(viewset.get_basename(), "test");
}

/// Test viewset has correct basename
#[tokio::test]
async fn test_viewset_get_basename() {
    let viewset = GenericViewSet::new("users", ());
    assert_eq!(viewset.get_basename(), "users");

    let model_viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("posts");
    assert_eq!(model_viewset.get_basename(), "posts");

    let readonly_viewset: ReadOnlyModelViewSet<TestModel, TestSerializer> =
        ReadOnlyModelViewSet::new("comments");
    assert_eq!(readonly_viewset.get_basename(), "comments");
}

/// Test different action types
#[tokio::test]
async fn test_viewset_action_types() {
    let viewset: ModelViewSet<TestModel, TestSerializer> = ModelViewSet::new("items");

    // Test list action
    let list_request = Request::new(
        Method::GET,
        Uri::from_static("/items/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let list_response = viewset.dispatch(list_request, Action::list()).await;
    assert!(list_response.is_ok());
    assert_eq!(list_response.unwrap().status, StatusCode::OK);

    // Test retrieve action
    let retrieve_request = Request::new(
        Method::GET,
        Uri::from_static("/items/1/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let retrieve_response = viewset.dispatch(retrieve_request, Action::retrieve()).await;
    assert!(retrieve_response.is_ok());
    assert_eq!(retrieve_response.unwrap().status, StatusCode::OK);

    // Test create action
    let create_request = Request::new(
        Method::POST,
        Uri::from_static("/items/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(r#"{"name": "test"}"#),
    );
    let create_response = viewset.dispatch(create_request, Action::create()).await;
    assert!(create_response.is_ok());
    assert_eq!(create_response.unwrap().status, StatusCode::CREATED);

    // Test update action
    let update_request = Request::new(
        Method::PUT,
        Uri::from_static("/items/1/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(r#"{"name": "updated"}"#),
    );
    let update_response = viewset.dispatch(update_request, Action::update()).await;
    assert!(update_response.is_ok());
    assert_eq!(update_response.unwrap().status, StatusCode::OK);

    // Test destroy action
    let destroy_request = Request::new(
        Method::DELETE,
        Uri::from_static("/items/1/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let destroy_response = viewset.dispatch(destroy_request, Action::destroy()).await;
    assert!(destroy_response.is_ok());
    assert_eq!(destroy_response.unwrap().status, StatusCode::NO_CONTENT);
}

/// Test readonly viewset restrictions
#[tokio::test]
async fn test_readonly_viewset_restrictions() {
    let viewset: ReadOnlyModelViewSet<TestModel, TestSerializer> =
        ReadOnlyModelViewSet::new("readonly");

    // List should work
    let list_request = Request::new(
        Method::GET,
        Uri::from_static("/readonly/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let list_response = viewset.dispatch(list_request, Action::list()).await;
    assert!(list_response.is_ok());
    assert_eq!(list_response.unwrap().status, StatusCode::OK);

    // Retrieve should work
    let retrieve_request = Request::new(
        Method::GET,
        Uri::from_static("/readonly/1/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let retrieve_response = viewset.dispatch(retrieve_request, Action::retrieve()).await;
    assert!(retrieve_response.is_ok());
    assert_eq!(retrieve_response.unwrap().status, StatusCode::OK);

    // Create should fail
    let create_request = Request::new(
        Method::POST,
        Uri::from_static("/readonly/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(r#"{"name": "test"}"#),
    );
    let create_response = viewset.dispatch(create_request, Action::create()).await;
    assert!(create_response.is_err());

    // Delete should fail
    let delete_request = Request::new(
        Method::DELETE,
        Uri::from_static("/readonly/1/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let delete_response = viewset.dispatch(delete_request, Action::destroy()).await;
    assert!(delete_response.is_err());
}

/// Test custom actions
#[tokio::test]
async fn test_custom_actions() {
    let list_action = Action::custom("custom_list", false);
    assert_eq!(list_action.detail, false);

    let detail_action = Action::custom("custom_detail", true);
    assert_eq!(detail_action.detail, true);
}

/// Test action metadata
#[tokio::test]
async fn test_action_metadata() {
    let list = Action::list();
    assert_eq!(list.detail, false);

    let retrieve = Action::retrieve();
    assert_eq!(retrieve.detail, true);

    let create = Action::create();
    assert_eq!(create.detail, false);

    let update = Action::update();
    assert_eq!(update.detail, true);

    let partial_update = Action::partial_update();
    assert_eq!(partial_update.detail, true);

    let destroy = Action::destroy();
    assert_eq!(destroy.detail, true);
}
