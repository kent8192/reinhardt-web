//! Runtime behavior tests for macros
//!
//! Tests that verify the actual runtime behavior of macro-generated code,
//! including function calls, parameter passing, and error handling.

#[allow(dead_code)]
mod test_types {
	use std::fmt;

	#[derive(Debug, Clone, PartialEq)]
	pub struct Request {
		pub method: String,
		pub path: String,
	}

	impl Request {
		pub fn new(method: &str, path: &str) -> Self {
			Self {
				method: method.to_string(),
				path: path.to_string(),
			}
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	pub struct Response {
		pub status: u16,
		pub body: String,
	}

	impl Response {
		pub fn ok() -> Self {
			Self {
				status: 200,
				body: "OK".to_string(),
			}
		}

		pub fn created() -> Self {
			Self {
				status: 201,
				body: "Created".to_string(),
			}
		}

		pub fn with_body(body: String) -> Self {
			Self { status: 200, body }
		}
	}

	#[derive(Debug, Clone, PartialEq)]
	pub struct RuntimeError(pub String);

	impl fmt::Display for RuntimeError {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			write!(f, "{}", self.0)
		}
	}

	impl std::error::Error for RuntimeError {}
}

use reinhardt_macros::{action, api_view, get, post};
use test_types::*;

// Test 1: api_view function can actually be called
#[api_view(methods = "GET")]
async fn callable_view(request: Request) -> Result<Response, RuntimeError> {
	assert_eq!(request.method, "GET");
	Ok(Response::ok())
}

#[tokio::test]
async fn test_api_view_callable() {
	let request = Request::new("GET", "/test");
	let result = callable_view(request).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().status, 200);
}

// Test 2: Parameters are correctly passed through
#[get("/users/{id}")]
async fn view_with_param(request: Request, id: i64) -> Result<Response, RuntimeError> {
	assert_eq!(request.path, "/users/{id}");
	Ok(Response::with_body(format!("User ID: {}", id)))
}

#[tokio::test]
async fn test_parameter_passing() {
	let request = Request::new("GET", "/users/{id}");
	let result = view_with_param(request, 42).await;
	assert!(result.is_ok());
	let response = result.unwrap();
	assert_eq!(response.body, "User ID: 42");
}

// Test 3: Multiple parameters work correctly
#[get("/users/{user_id}/posts/{post_id}")]
async fn view_with_multiple_params(
	_request: Request,
	user_id: i64,
	post_id: i64,
) -> Result<Response, RuntimeError> {
	Ok(Response::with_body(format!(
		"User: {}, Post: {}",
		user_id, post_id
	)))
}

#[tokio::test]
async fn test_macros_multiple_parameters() {
	let request = Request::new("GET", "/users/1/posts/2");
	let result = view_with_multiple_params(request, 1, 2).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().body, "User: 1, Post: 2");
}

// Test 4: Error handling works correctly
#[post("/items")]
async fn view_with_error(_request: Request) -> Result<Response, RuntimeError> {
	Err(RuntimeError("Something went wrong".to_string()))
}

#[tokio::test]
async fn test_macros_runtime_error_handling() {
	let request = Request::new("POST", "/items");
	let result = view_with_error(request).await;
	assert!(result.is_err());
	assert_eq!(result.unwrap_err().0, "Something went wrong");
}

// Test 5: ViewSet actions can be called
struct TestViewSet {
	name: String,
}

impl TestViewSet {
	fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
		}
	}

	#[action(methods = "POST", detail = true)]
	async fn activate(&self, _request: Request, pk: i64) -> Result<Response, RuntimeError> {
		Ok(Response::with_body(format!(
			"{} activated item {}",
			self.name, pk
		)))
	}

	#[action(methods = "GET", detail = false)]
	async fn list_all(&self, _request: Request) -> Result<Response, RuntimeError> {
		Ok(Response::with_body(format!("{} listing all", self.name)))
	}
}

#[tokio::test]
async fn test_viewset_detail_action() {
	let viewset = TestViewSet::new("TestSet");
	let request = Request::new("POST", "/items/1/activate");
	let result = viewset.activate(request, 1).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().body, "TestSet activated item 1");
}

#[tokio::test]
async fn test_viewset_list_action() {
	let viewset = TestViewSet::new("TestSet");
	let request = Request::new("GET", "/items/list_all");
	let result = viewset.list_all(request).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().body, "TestSet listing all");
}

// Test 6: Mutable state in ViewSet
struct StatefulViewSet {
	counter: std::sync::Arc<std::sync::Mutex<i32>>,
}

impl StatefulViewSet {
	fn new() -> Self {
		Self {
			counter: std::sync::Arc::new(std::sync::Mutex::new(0)),
		}
	}

	#[action(methods = "POST", detail = false)]
	async fn increment(&self, _request: Request) -> Result<Response, RuntimeError> {
		let mut counter = self.counter.lock().unwrap();
		*counter += 1;
		Ok(Response::with_body(format!("Counter: {}", *counter)))
	}
}

#[tokio::test]
async fn test_stateful_viewset() {
	let viewset = StatefulViewSet::new();
	let request1 = Request::new("POST", "/increment");
	let result1 = viewset.increment(request1).await;
	assert_eq!(result1.unwrap().body, "Counter: 1");

	let request2 = Request::new("POST", "/increment");
	let result2 = viewset.increment(request2).await;
	assert_eq!(result2.unwrap().body, "Counter: 2");
}

// Test 7: Async operations within handlers
#[get("/async-operation")]
async fn view_with_async_ops(_request: Request) -> Result<Response, RuntimeError> {
	// Simulate async database call
	async fn fetch_data() -> Result<String, RuntimeError> {
		tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
		Ok("data".to_string())
	}

	let data = fetch_data().await?;
	Ok(Response::with_body(format!("Fetched: {}", data)))
}

#[tokio::test]
async fn test_async_operations() {
	let request = Request::new("GET", "/async-operation");
	let result = view_with_async_ops(request).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().body, "Fetched: data");
}

// Test 8: Optional parameters
#[get("/search")]
async fn view_with_optional(
	_request: Request,
	query: Option<String>,
) -> Result<Response, RuntimeError> {
	let query_str = query.unwrap_or_else(|| "default".to_string());
	Ok(Response::with_body(format!("Query: {}", query_str)))
}

#[tokio::test]
async fn test_optional_with_value() {
	let request = Request::new("GET", "/search");
	let result = view_with_optional(request, Some("test".to_string())).await;
	assert_eq!(result.unwrap().body, "Query: test");
}

#[tokio::test]
async fn test_optional_without_value() {
	let request = Request::new("GET", "/search");
	let result = view_with_optional(request, None).await;
	assert_eq!(result.unwrap().body, "Query: default");
}

// Test 9: Borrowed parameters
#[post("/echo")]
async fn view_with_borrowed(_request: Request, message: &str) -> Result<Response, RuntimeError> {
	Ok(Response::with_body(format!("Echo: {}", message)))
}

#[tokio::test]
async fn test_borrowed_parameter() {
	let request = Request::new("POST", "/echo");
	let message = "Hello, World!";
	let result = view_with_borrowed(request, message).await;
	assert_eq!(result.unwrap().body, "Echo: Hello, World!");
}

// Test 10: Complex return types
#[get("/complex")]
async fn view_with_complex_return(_request: Request) -> Result<Vec<(String, i32)>, RuntimeError> {
	Ok(vec![
		("item1".to_string(), 1),
		("item2".to_string(), 2),
		("item3".to_string(), 3),
	])
}

#[tokio::test]
async fn test_macros_runtime_complex_return() {
	let request = Request::new("GET", "/complex");
	let result = view_with_complex_return(request).await;
	assert!(result.is_ok());
	let items = result.unwrap();
	assert_eq!(items.len(), 3);
	assert_eq!(items[0].0, "item1");
	assert_eq!(items[0].1, 1);
}

// Test 11: Chaining async operations
#[get("/chain")]
async fn view_with_chained_ops(_request: Request) -> Result<Response, RuntimeError> {
	async fn step1() -> Result<i32, RuntimeError> {
		Ok(10)
	}

	async fn step2(x: i32) -> Result<i32, RuntimeError> {
		Ok(x * 2)
	}

	async fn step3(x: i32) -> Result<String, RuntimeError> {
		Ok(format!("Result: {}", x))
	}

	let value = step1().await?;
	let doubled = step2(value).await?;
	let result = step3(doubled).await?;

	Ok(Response::with_body(result))
}

#[tokio::test]
async fn test_chained_async_operations() {
	let request = Request::new("GET", "/chain");
	let result = view_with_chained_ops(request).await;
	assert_eq!(result.unwrap().body, "Result: 20");
}

// Test 12: ViewSet with custom URL path
struct CustomUrlViewSet;

impl CustomUrlViewSet {
	#[action(methods = "POST", detail = true, url_path = "/custom-action")]
	async fn my_action(&self, _request: Request, pk: i64) -> Result<Response, RuntimeError> {
		Ok(Response::with_body(format!("Custom action for {}", pk)))
	}
}

#[tokio::test]
async fn test_custom_url_path() {
	let viewset = CustomUrlViewSet;
	let request = Request::new("POST", "/items/1/custom-action");
	let result = viewset.my_action(request, 1).await;
	assert_eq!(result.unwrap().body, "Custom action for 1");
}
