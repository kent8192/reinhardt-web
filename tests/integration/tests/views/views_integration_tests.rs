//! Integration tests for views functionality
//!
//! Tests that involve multiple reinhardt crates working together

use reinhardt_apps::{Error, Request, Response, Result};
use reinhardt_orm::Model;
use reinhardt_serializers::{JsonSerializer, Serializer};
use reinhardt_views::{test_utils::*, Context, View};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Integration test view that uses multiple crates
pub struct IntegrationTestView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<T> + Send + Sync,
{
    objects: Vec<T>,
    _serializer: PhantomData<S>,
}

impl<T, S> IntegrationTestView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
    S: Serializer<T> + Send + Sync,
{
    pub fn new(objects: Vec<T>) -> Self {
        Self {
            objects,
            _serializer: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T, S> View for IntegrationTestView<T, S>
where
    T: Model + Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    S: Serializer<T> + Send + Sync + Default + 'static,
{
    async fn dispatch(&self, request: Request) -> Result<Response> {
        match request.method {
            hyper::Method::GET => {
                // Test serialization integration
                let serializer = S::default();
                let serialized_objects: Result<Vec<_>> = self
                    .objects
                    .iter()
                    .map(|obj| serializer.serialize(obj))
                    .collect();

                let serialized_objects = serialized_objects?;

                // Test context integration
                let mut context = Context::new();
                context.insert(
                    "objects".to_string(),
                    serde_json::to_value(&serialized_objects).unwrap(),
                );
                context.insert(
                    "count".to_string(),
                    serde_json::Value::Number((self.objects.len() as i64).into()),
                );

                // Build response with context
                let mut response_data = serde_json::Map::new();
                response_data.insert(
                    "results".to_string(),
                    serde_json::to_value(&serialized_objects).unwrap(),
                );
                response_data.insert(
                    "context".to_string(),
                    serde_json::to_value(context).unwrap(),
                );

                Response::ok().with_json(&response_data)
            }
            _ => Err(Error::Validation(format!(
                "Method {} not allowed",
                request.method
            ))),
        }
    }
}

/// Integration test view that tests error handling across crates
pub struct ErrorIntegrationView;

#[async_trait::async_trait]
impl View for ErrorIntegrationView {
    async fn dispatch(&self, request: Request) -> Result<Response> {
        // Test error propagation from different crates
        match request.method {
            hyper::Method::GET => {
                // Simulate a validation error
                Err(Error::Validation(
                    "Integration test validation error".to_string(),
                ))
            }
            hyper::Method::POST => {
                // Simulate a not found error
                Err(Error::NotFound(
                    "Integration test not found error".to_string(),
                ))
            }
            hyper::Method::PUT => {
                // Simulate an internal error
                Err(Error::Internal(
                    "Integration test internal error".to_string(),
                ))
            }
            _ => Err(Error::Validation(format!(
                "Method {} not allowed",
                request.method
            ))),
        }
    }
}

/// Integration test view that tests serialization with complex data
pub struct SerializationIntegrationView;

#[async_trait::async_trait]
impl View for SerializationIntegrationView {
    async fn dispatch(&self, request: Request) -> Result<Response> {
        match request.method {
            hyper::Method::GET => {
                // Test complex serialization
                let complex_data = HashMap::from([
                    (
                        "string".to_string(),
                        serde_json::Value::String("test".to_string()),
                    ),
                    ("number".to_string(), serde_json::Value::Number(42.into())),
                    ("boolean".to_string(), serde_json::Value::Bool(true)),
                    (
                        "array".to_string(),
                        serde_json::Value::Array(vec![
                            serde_json::Value::String("item1".to_string()),
                            serde_json::Value::String("item2".to_string()),
                        ]),
                    ),
                    (
                        "object".to_string(),
                        serde_json::json!({
                            "nested": "value",
                            "count": 5
                        }),
                    ),
                ]);

                Response::ok().with_json(&complex_data)
            }
            hyper::Method::POST => {
                // Test deserialization
                let data: serde_json::Value = serde_json::from_slice(&request.body)
                    .map_err(|_| Error::Validation("Invalid JSON".to_string()))?;

                // Echo back the data
                Response::ok().with_json(&data)
            }
            _ => Err(Error::Validation(format!(
                "Method {} not allowed",
                request.method
            ))),
        }
    }
}

/// Integration test view that tests context handling
pub struct ContextIntegrationView;

#[async_trait::async_trait]
impl View for ContextIntegrationView {
    async fn dispatch(&self, request: Request) -> Result<Response> {
        match request.method {
            hyper::Method::GET => {
                // Test context creation and manipulation
                let mut context = Context::new();
                context.insert(
                    "request_method".to_string(),
                    serde_json::Value::String(request.method.to_string()),
                );
                context.insert(
                    "request_path".to_string(),
                    serde_json::Value::String(request.uri.path().to_string()),
                );
                context.insert(
                    "timestamp".to_string(),
                    serde_json::Value::String("2023-01-01T00:00:00Z".to_string()),
                );

                // Add query parameters to context
                if !request.query_params.is_empty() {
                    context.insert(
                        "query_params".to_string(),
                        serde_json::to_value(&request.query_params).unwrap(),
                    );
                }

                // Add headers to context
                let headers_map: HashMap<String, String> = request
                    .headers
                    .iter()
                    .filter_map(|(name, value)| {
                        value
                            .to_str()
                            .ok()
                            .map(|v| (name.to_string(), v.to_string()))
                    })
                    .collect();

                if !headers_map.is_empty() {
                    context.insert(
                        "headers".to_string(),
                        serde_json::to_value(&headers_map).unwrap(),
                    );
                }

                Response::ok().with_json(&serde_json::to_value(context).unwrap())
            }
            _ => Err(Error::Validation(format!(
                "Method {} not allowed",
                request.method
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Method;

    #[tokio::test]
    async fn test_integration_view_basic() {
        let objects = create_test_objects();
        let view = IntegrationTestView::<TestModel, JsonSerializer<TestModel>>::new(objects);
        let request = create_request(Method::GET, "/integration/", None, None, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(&response, "count", &serde_json::Value::Number(3.into()));
    }

    #[tokio::test]
    async fn test_integration_view_with_api_objects() {
        let objects = create_api_test_objects();
        let view = IntegrationTestView::<ApiTestModel, JsonSerializer<ApiTestModel>>::new(objects);
        let request = create_request(Method::GET, "/integration/", None, None, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(&response, "count", &serde_json::Value::Number(3.into()));
    }

    #[tokio::test]
    async fn test_integration_view_post_not_allowed() {
        let objects = create_test_objects();
        let view = IntegrationTestView::<TestModel, JsonSerializer<TestModel>>::new(objects);
        let request = create_request(Method::POST, "/integration/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_error_integration_view_validation() {
        let view = ErrorIntegrationView;
        let request = create_request(Method::GET, "/error/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_error_integration_view_not_found() {
        let view = ErrorIntegrationView;
        let request = create_request(Method::POST, "/error/", None, None, None);
        let result = view.dispatch(request).await;

        assert_not_found_error(result);
    }

    #[tokio::test]
    async fn test_error_integration_view_internal() {
        let view = ErrorIntegrationView;
        let request = create_request(Method::PUT, "/error/", None, None, None);
        let result = view.dispatch(request).await;

        assert_internal_error(result);
    }

    #[tokio::test]
    async fn test_serialization_integration_view_get() {
        let view = SerializationIntegrationView;
        let request = create_request(Method::GET, "/serialization/", None, None, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(
            &response,
            "string",
            &serde_json::Value::String("test".to_string()),
        );
        assert_json_response_contains(&response, "number", &serde_json::Value::Number(42.into()));
        assert_json_response_contains(&response, "boolean", &serde_json::Value::Bool(true));
    }

    #[tokio::test]
    async fn test_serialization_integration_view_post() {
        let view = SerializationIntegrationView;
        let json_data = serde_json::json!({
            "test": "data",
            "number": 123,
            "nested": {
                "value": "test"
            }
        });
        let request = create_json_request(Method::POST, "/serialization/", &json_data);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(
            &response,
            "test",
            &serde_json::Value::String("data".to_string()),
        );
        assert_json_response_contains(&response, "number", &serde_json::Value::Number(123.into()));
    }

    #[tokio::test]
    async fn test_serialization_integration_view_invalid_json() {
        let view = SerializationIntegrationView;
        let body = bytes::Bytes::from("invalid json");
        let request = create_request(Method::POST, "/serialization/", None, None, Some(body));
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_context_integration_view_basic() {
        let view = ContextIntegrationView;
        let request = create_request(Method::GET, "/context/", None, None, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(
            &response,
            "request_method",
            &serde_json::Value::String("GET".to_string()),
        );
        assert_json_response_contains(
            &response,
            "request_path",
            &serde_json::Value::String("/context/".to_string()),
        );
    }

    #[tokio::test]
    async fn test_context_integration_view_with_query_params() {
        let view = ContextIntegrationView;
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());
        query_params.insert("limit".to_string(), "10".to_string());

        let request = create_request(Method::GET, "/context/", Some(query_params), None, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(
            &response,
            "query_params",
            &serde_json::json!({
                "page": "1",
                "limit": "10"
            }),
        );
    }

    #[tokio::test]
    async fn test_context_integration_view_with_headers() {
        let view = ContextIntegrationView;
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "Test Agent".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());

        let request = create_request_with_headers(Method::GET, "/context/", headers, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(
            &response,
            "headers",
            &serde_json::json!({
                "user-agent": "Test Agent",
                "accept": "application/json"
            }),
        );
    }

    #[tokio::test]
    async fn test_context_integration_view_post_not_allowed() {
        let view = ContextIntegrationView;
        let request = create_request(Method::POST, "/context/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_integration_view_empty_objects() {
        let view = IntegrationTestView::<TestModel, JsonSerializer<TestModel>>::new(Vec::new());
        let request = create_request(Method::GET, "/integration/", None, None, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(&response, "count", &serde_json::Value::Number(0.into()));
    }

    #[tokio::test]
    async fn test_integration_view_large_dataset() {
        let objects = create_large_test_objects(1000);
        let view = IntegrationTestView::<TestModel, JsonSerializer<TestModel>>::new(objects);
        let request = create_request(Method::GET, "/integration/", None, None, None);
        let response = view.dispatch(request).await.unwrap();

        assert_response_status(&response, StatusCode::OK);
        assert_json_response_contains(&response, "count", &serde_json::Value::Number(1000.into()));
    }

    #[tokio::test]
    async fn test_serialization_integration_view_put_not_allowed() {
        let view = SerializationIntegrationView;
        let request = create_request(Method::PUT, "/serialization/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_serialization_integration_view_patch_not_allowed() {
        let view = SerializationIntegrationView;
        let request = create_request(Method::PATCH, "/serialization/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_serialization_integration_view_delete_not_allowed() {
        let view = SerializationIntegrationView;
        let request = create_request(Method::DELETE, "/serialization/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_error_integration_view_head_not_allowed() {
        let view = ErrorIntegrationView;
        let request = create_request(Method::HEAD, "/error/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }

    #[tokio::test]
    async fn test_error_integration_view_options_not_allowed() {
        let view = ErrorIntegrationView;
        let request = create_request(Method::OPTIONS, "/error/", None, None, None);
        let result = view.dispatch(request).await;

        assert_validation_error(result);
    }
}
